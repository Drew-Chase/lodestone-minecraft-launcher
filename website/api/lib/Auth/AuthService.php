<?php

declare(strict_types=1);

namespace app\Auth;

use app\Config\Config;
use app\Db\Database;
use app\Devices\Devices;
use app\Mail\AccountMail;
use app\Mail\Mailer;
use app\Mail\Outbox;
use app\Users\UserRepository;
use DateTimeImmutable;
use DateTimeZone;
use PDOException;

/**
 * Account and session rules. Every method throws AuthException with the code and status from the README.
 * Nothing here knows about HTTP.
 */
final class AuthService
{
    private const VERIFY_TTL_SECONDS = 86400;
    private const RESET_TTL_SECONDS = 3600;
    private const FINISH_TTL_SECONDS = 86400;
    private const LOGIN_LIMIT = 5;
    private const LOGIN_WINDOW_SECONDS = 900;
    private const WEB_CLIENT = 'web';

    public function __construct(
        private readonly UserRepository $users,
        private readonly RefreshTokens $refresh,
        private readonly EmailTokens $emailTokens,
        private readonly RateLimiter $limiter,
        private readonly AuditLog $audit,
        private readonly PasswordHasher $hasher,
        private readonly PasswordPolicy $policy,
        private readonly JwtIssuer $jwt,
        private readonly Mailer $mailer,
        private readonly string $appUrl,
    ) {
    }

    /**
     * Account mail goes through an Outbox, which index.php flushes after the response has been sent.
     */
    public static function fromConfig(): self
    {
        return new self(
            new UserRepository(),
            new RefreshTokens(),
            new EmailTokens(),
            new RateLimiter(),
            new AuditLog(),
            new PasswordHasher(),
            new PasswordPolicy(self::breachChecker()),
            JwtIssuer::fromConfig(),
            new Outbox(),
            rtrim(Config::need('APP_URL'), '/'),
        );
    }

    /**
     * phpunit.xml.dist sets PASSWORD_BREACH_CHECK=off, so the suite never calls the Pwned Passwords service.
     */
    private static function breachChecker(): BreachChecker
    {
        return Config::get('PASSWORD_BREACH_CHECK', 'on') === 'off'
            ? new NullBreachChecker()
            : new HibpBreachChecker();
    }

    /**
     * @throws AuthException invalid_email (400)
     */
    public function normalizeEmail(string $email): string
    {
        $normalized = mb_strtolower(trim($email), 'UTF-8');

        if ($normalized === ''
            || strlen($normalized) > 254
            || filter_var($normalized, FILTER_VALIDATE_EMAIL) === false
        ) {
            throw new AuthException('invalid_email', 400);
        }

        return $normalized;
    }

    /**
     * Trims the name. An empty name becomes null.
     *
     * @throws AuthException invalid_display_name (400)
     */
    public function validDisplayName(?string $name): ?string
    {
        if ($name === null) {
            return null;
        }

        $trimmed = trim($name);
        if ($trimmed === '') {
            return null;
        }

        if (mb_strlen($trimmed, 'UTF-8') > 64 || preg_match('/[\x00-\x1F\x7F]/u', $trimmed) !== 0) {
            throw new AuthException('invalid_display_name', 400);
        }

        return $trimmed;
    }

    /**
     * Answers the same whether or not the address exists. Mail failures are logged, never returned.
     */
    public function register(string $email, string $password, ?string $displayName, string $ip): void
    {
        $this->hitOrThrow('register:ip:' . $ip, 10, 3600);

        $normalized = $this->normalizeEmail($email);
        $name = $this->validDisplayName($displayName);
        $this->policy->check($password);

        $existing = $this->users->findByEmail($normalized);
        if ($existing !== null) {
            $this->hasher->burnTime();

            if ($existing['deleted_at'] !== null) {
                return;
            }

            if ($existing['email_verified_at'] !== null) {
                $this->sendMail($normalized, AccountMail::alreadyRegistered($this->appUrl));
            } else {
                // The password of an unverified account may have been set by someone else, so it is left alone.
                $this->issueFinishRegistration((int) $existing['id'], $normalized);
            }

            return;
        }

        $hash = $this->hasher->hash($password);

        try {
            $userId = $this->users->create($normalized, $hash, $name);
        } catch (PDOException $error) {
            // A concurrent register won the unique email. Answer as if this one had found an existing account.
            if ($error->getCode() === '23000') {
                return;
            }

            throw $error;
        }

        $this->issueVerification($userId, $normalized);
        $this->audit->record($userId, 'register', $ip);
    }

    /**
     * @throws AuthException rate_limited (429), invalid_token (400) or token_expired (400)
     */
    public function verifyEmail(string $token, string $ip): void
    {
        $this->hitOrThrow('verify:ip:' . $ip, 30, 900);

        $userId = $this->emailTokens->consume($token, EmailTokens::VERIFY);
        $this->users->markVerified($userId);
        $this->refresh->revokeAllForUser($userId);
        $this->audit->record($userId, 'verify_email', $ip);
    }

    /**
     * Sends a finish-registration link only for an unverified account. Otherwise does nothing.
     */
    public function resendVerification(string $email, string $ip): void
    {
        $this->hitOrThrow('resend:ip:' . $ip, 10, 3600);

        $normalized = $this->normalizeEmail($email);
        $this->hitOrThrow('resend:email:' . $normalized, 3, 3600);

        $user = $this->users->findByEmail($normalized);
        if ($user !== null && $user['deleted_at'] === null && $user['email_verified_at'] === null) {
            $this->issueFinishRegistration((int) $user['id'], $normalized);
        }
    }

    /**
     * @return array{user: array<string, mixed>, access_token: string, expires_in: int, refresh_token: string, family_id: string}
     * @throws AuthException rate_limited (429) or invalid_credentials (401)
     */
    public function login(string $email, string $password, string $ip): array
    {
        // A malformed address is treated as an unknown user, so its text is only used as a bucket key.
        $key = mb_strtolower(trim($email), 'UTF-8');
        $emailBucket = 'login:email:' . $key;
        $ipBucket = 'login:ip:' . $ip;

        // Each attempt is counted before its password is checked, so attempts that run at once each count. A check made before
        // the count let a burst of parallel attempts all pass it. A successful attempt gives its count back to the address.
        $this->hitOrThrow($emailBucket, self::LOGIN_LIMIT, self::LOGIN_WINDOW_SECONDS);
        $this->hitOrThrow($ipBucket, self::LOGIN_LIMIT, self::LOGIN_WINDOW_SECONDS);

        $user = $this->findLoginUser($key);
        if ($user === null || $user['deleted_at'] !== null || $user['password_hash'] === null) {
            $this->hasher->burnTime();
            $this->failLogin($ip, $user === null ? null : (int) $user['id']);
        }

        $userId = (int) $user['id'];
        $storedHash = (string) $user['password_hash'];

        if (!$this->hasher->verify($password, $storedHash)) {
            $this->failLogin($ip, $userId);
        }

        $this->limiter->clear($emailBucket);
        $this->limiter->release($ipBucket, self::LOGIN_WINDOW_SECONDS);

        if ($this->hasher->needsRehash($storedHash)) {
            $this->users->setPasswordHash($userId, $this->hasher->hash($password));
        }

        $session = $this->openSession($userId, self::WEB_CLIENT);
        $this->audit->record($userId, 'login', $ip);

        return ['user' => $this->users->toPublic($user)] + $session;
    }

    /**
     * Rotates the presented refresh token and issues a new access token for its family.
     *
     * @return array{user: array<string, mixed>, access_token: string, expires_in: int, refresh_token: string, family_id: string}
     * @throws AuthException refresh_invalid, refresh_reused or refresh_expired (401)
     */
    public function refresh(string $presentedToken, string $client, string $ip): array
    {
        $rotated = $this->refresh->rotate($presentedToken, $client, $ip);
        $userId = $rotated['user_id'];

        $user = $this->users->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            $this->refresh->revokeFamily($rotated['family_id']);
            throw new AuthException('refresh_invalid', 401);
        }

        $access = $this->jwt->issue($userId, $rotated['family_id']);
        $this->audit->record($userId, 'refresh', $ip);

        if ($rotated['device_id'] !== null) {
            (new Devices())->touch($rotated['device_id']);
        }

        return [
            'user' => $this->users->toPublic($user),
            'access_token' => $access['token'],
            'expires_in' => $access['expires_in'],
            'refresh_token' => $rotated['token'],
            'family_id' => $rotated['family_id'],
        ];
    }

    /**
     * Revokes the session of the presented refresh token. An unknown or malformed token is ignored. A token that
     * belongs to another client is refused, and nothing is revoked.
     *
     * @param ?string $client the client the token must belong to, or null to accept any client
     * @throws AuthException refresh_invalid (401) when the token belongs to another client
     */
    public function logout(string $presentedToken, string $ip, ?string $client = null): void
    {
        if ($client !== null) {
            $owner = $this->clientOf($presentedToken);
            if ($owner !== null && $owner !== $client) {
                throw new AuthException('refresh_invalid', 401);
            }
        }

        $revoked = $this->refresh->revokeByToken($presentedToken);

        if ($revoked !== null) {
            $this->audit->record($revoked['user_id'], 'logout', $ip);
        }
    }

    /** The client a refresh token was issued to, or null when the token is malformed or unknown. */
    private function clientOf(string $presentedToken): ?string
    {
        if (!Tokens::isWellFormed($presentedToken)) {
            return null;
        }

        $statement = Database::pdo()->prepare('SELECT client FROM refresh_tokens WHERE token_hash = ?');
        $statement->execute([Tokens::hash($presentedToken)]);
        $client = $statement->fetchColumn();

        return $client === false ? null : (string) $client;
    }

    /**
     * Sends a reset link only for an existing, undeleted account. Answers the same otherwise.
     */
    public function requestPasswordReset(string $email, string $ip): void
    {
        $this->hitOrThrow('forgot:ip:' . $ip, 10, 3600);

        $normalized = $this->normalizeEmail($email);
        $this->hitOrThrow('forgot:email:' . $normalized, 3, 3600);

        $user = $this->users->findByEmail($normalized);
        if ($user === null || $user['deleted_at'] !== null) {
            return;
        }

        $userId = (int) $user['id'];
        $token = $this->emailTokens->create($userId, EmailTokens::RESET, self::RESET_TTL_SECONDS);
        $this->sendMail($normalized, AccountMail::passwordReset($this->appUrl, $token));
        $this->audit->record($userId, 'password_reset_requested', $ip);
    }

    /**
     * The policy runs before the token is spent, so a weak password does not use up the link.
     * A reset proves the address, and it revokes every session.
     *
     * @throws AuthException rate_limited, password policy codes (400), invalid_token or token_expired (400)
     */
    public function resetPassword(string $token, string $newPassword, string $ip): void
    {
        $this->hitOrThrow('reset:ip:' . $ip, 30, 900);
        $this->policy->check($newPassword);

        $userId = $this->emailTokens->consume($token, EmailTokens::RESET);
        $this->users->setPasswordHash($userId, $this->hasher->hash($newPassword));
        $this->users->markVerified($userId);
        $this->refresh->revokeAllForUser($userId);
        $this->audit->record($userId, 'password_reset', $ip);
    }

    /**
     * Keeps the current session (its family) and revokes the others.
     *
     * @throws AuthException rate_limited (429), unauthorized (401), invalid_credentials (401) or password policy codes (400)
     */
    public function changePassword(
        int $userId,
        string $currentPassword,
        string $newPassword,
        string $familyId,
        string $ip,
    ): void {
        $bucket = 'changepw:user:' . $userId;
        // Counted before the current password is checked, as a login is.
        $this->hitOrThrow($bucket, self::LOGIN_LIMIT, self::LOGIN_WINDOW_SECONDS);

        $user = $this->users->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            throw new AuthException('unauthorized', 401);
        }

        $storedHash = $user['password_hash'];
        if ($storedHash === null || !$this->hasher->verify($currentPassword, (string) $storedHash)) {
            $this->audit->record($userId, 'change_password_failed', $ip);
            throw new AuthException('invalid_credentials', 401);
        }
        // The current password was right, so this attempt does not count against the account.
        $this->limiter->release($bucket, self::LOGIN_WINDOW_SECONDS);

        $this->policy->check($newPassword);
        $this->users->setPasswordHash($userId, $this->hasher->hash($newPassword));
        $this->refresh->revokeAllExceptFamily($userId, $familyId);
        $this->audit->record($userId, 'password_changed', $ip);
    }

    /**
     * @return array<string, mixed>
     * @throws AuthException unauthorized (401) when the user is missing or deleted
     */
    public function currentUser(int $userId): array
    {
        $user = $this->users->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            throw new AuthException('unauthorized', 401);
        }

        return $this->users->toPublic($user);
    }

    /**
     * The account's active sessions. The session of the caller's access token is marked current.
     *
     * @return array{sessions: list<array{id: string, client: string, created_at: string, last_used_at: string, current: bool}>}
     */
    public function listSessions(int $userId, string $currentFamilyId): array
    {
        $sessions = [];
        foreach ($this->refresh->listActiveForUser($userId) as $row) {
            $sessions[] = [
                'id' => $row['family_id'],
                'client' => $row['client'],
                'created_at' => $this->isoUtc($row['created_at']),
                'last_used_at' => $this->isoUtc($row['last_used_at']),
                'current' => $row['family_id'] === $currentFamilyId,
            ];
        }

        return ['sessions' => $sessions];
    }

    /**
     * Revokes one of the account's sessions. The caller's own session is ended by logout instead.
     *
     * @throws AuthException not_found (404) for a malformed id or a session that is not active,
     *                       cannot_revoke_current (409) for the caller's own session
     */
    public function revokeSession(int $userId, string $familyId, string $currentFamilyId, string $ip): void
    {
        if (preg_match('/^[a-f0-9]{32}$/D', $familyId) !== 1) {
            throw new AuthException('not_found', 404);
        }

        if ($familyId === $currentFamilyId) {
            throw new AuthException('cannot_revoke_current', 409);
        }

        if (!$this->refresh->familyIsActiveForUser($userId, $familyId)) {
            throw new AuthException('not_found', 404);
        }

        // The session can end between the check and the update, so an update that changed nothing is not found.
        if (!$this->refresh->revokeFamilyForUser($userId, $familyId)) {
            throw new AuthException('not_found', 404);
        }

        $this->audit->record($userId, 'session_revoked', $ip);
    }

    /**
     * Revokes every session of the account except the caller's. Returns how many active sessions were there.
     */
    public function revokeOtherSessions(int $userId, string $currentFamilyId, string $ip): int
    {
        $count = $this->refresh->countOtherActive($userId, $currentFamilyId);
        $this->refresh->revokeAllExceptFamily($userId, $currentFamilyId);
        $this->audit->record($userId, 'sessions_revoked_others', $ip);

        return $count;
    }

    /**
     * Counts one attempt and throws once the count passes the limit.
     *
     * @throws AuthException rate_limited (429)
     */
    private function hitOrThrow(string $bucket, int $limit, int $windowSeconds): void
    {
        if ($this->limiter->hit($bucket, $windowSeconds) > $limit) {
            throw new AuthException('rate_limited', 429, $this->limiter->retryAfter($windowSeconds));
        }
    }

    /**
     * @return array<string, mixed>|null Null when the address is malformed, so it is never looked up.
     */
    private function findLoginUser(string $key): ?array
    {
        if ($key === '' || strlen($key) > 254 || filter_var($key, FILTER_VALIDATE_EMAIL) === false) {
            return null;
        }

        return $this->users->findByEmail($key);
    }

    /**
     * Audits a failed login without the address, and throws. The attempt was counted before its password was checked.
     *
     * @throws AuthException invalid_credentials (401)
     */
    private function failLogin(string $ip, ?int $userId): never
    {
        $this->audit->record($userId, 'login_failed', $ip);

        throw new AuthException('invalid_credentials', 401);
    }

    /**
     * Starts a web session for an account that a single sign-on provider has just authenticated.
     *
     * @return array{user: array<string, mixed>, access_token: string, expires_in: int, refresh_token: string, family_id: string}
     * @throws AuthException unauthorized (401) when the account is missing or deleted
     */
    public function startSession(int $userId, string $ip): array
    {
        $user = $this->users->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            throw new AuthException('unauthorized', 401);
        }

        $session = $this->openSession($userId, self::WEB_CLIENT);
        $this->audit->record($userId, 'login_sso', $ip);

        return ['user' => $this->users->toPublic($user)] + $session;
    }

    /**
     * Starts a family for a fresh sign-in and issues its access token.
     *
     * @return array{access_token: string, expires_in: int, refresh_token: string, family_id: string}
     */
    private function openSession(int $userId, string $client): array
    {
        $refresh = $this->refresh->issue($userId, $client);
        $access = $this->jwt->issue($userId, $refresh['family_id']);

        return [
            'access_token' => $access['token'],
            'expires_in' => $access['expires_in'],
            'refresh_token' => $refresh['token'],
            'family_id' => $refresh['family_id'],
        ];
    }

    /** Formats DATETIME(6) text, stored in UTC, the way UserRepository::toPublic does. */
    private function isoUtc(string $databaseTime): string
    {
        return (new DateTimeImmutable($databaseTime, new DateTimeZone('UTC')))->format(DATE_ATOM);
    }

    private function issueVerification(int $userId, string $email): void
    {
        $token = $this->emailTokens->create($userId, EmailTokens::VERIFY, self::VERIFY_TTL_SECONDS);
        $this->sendMail($email, AccountMail::verification($this->appUrl, $token));
    }

    /**
     * A reset-purpose link for an unverified account that already exists. Spending it sets the password,
     * verifies the address and revokes every session, as a password reset does.
     */
    private function issueFinishRegistration(int $userId, string $email): void
    {
        $token = $this->emailTokens->create($userId, EmailTokens::RESET, self::FINISH_TTL_SECONDS);
        $this->sendMail($email, AccountMail::finishRegistration($this->appUrl, $token));
    }

    /**
     * Queues the mail. A failure is logged with the exception's class only. The address and the token are never logged.
     *
     * @param array{subject: string, text: string, html: string} $mail
     */
    private function sendMail(string $to, array $mail): void
    {
        try {
            $this->mailer->send($to, $mail['subject'], $mail['text'], $mail['html']);
        } catch (\Throwable $error) {
            error_log('Account email could not be sent: ' . $error::class);
        }
    }
}
