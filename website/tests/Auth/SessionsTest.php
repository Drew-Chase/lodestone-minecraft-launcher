<?php

declare(strict_types=1);

namespace tests\Auth;

use DateInterval;
use DateTimeImmutable;
use DateTimeZone;
use app\Auth\AuditLog;
use app\Auth\AuthException;
use app\Auth\AuthService;
use app\Auth\EmailTokens;
use app\Auth\JwtIssuer;
use app\Auth\NullBreachChecker;
use app\Auth\PasswordHasher;
use app\Auth\PasswordPolicy;
use app\Auth\RateLimiter;
use app\Auth\RefreshTokens;
use app\Mail\Mailer;
use app\Mail\MemoryMailer;
use app\Users\UserRepository;
use tests\Support\DatabaseTestCase;

/**
 * The session rules: listing, revoking one session and revoking the others.
 */
final class SessionsTest extends DatabaseTestCase
{
    private const EMAIL = 'pilot@example.test';
    private const OTHER_EMAIL = 'copilot@example.test';
    private const PASSWORD = 'correct horse battery';
    private const IP = '198.51.100.40';
    private const SECRET = 'SESSIONS_TEST_SECRET_0123456789abcdef';
    private const UNKNOWN_FAMILY = 'ffffffffffffffffffffffffffffffff';

    private MemoryMailer $mailer;
    private AuthService $auth;

    protected function setUp(): void
    {
        parent::setUp();

        $this->mailer = new MemoryMailer();
        $this->mailer->reset();
        $this->auth = $this->service($this->mailer);
    }

    public function testTwoLoginsGiveTwoSessionsAndExactlyOneIsCurrent(): void
    {
        $first = $this->registerAndLogin();
        $second = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $listed = $this->auth->listSessions($first['user']['id'], $first['family_id']);

        self::assertCount(2, $listed['sessions']);
        self::assertSame(1, $this->countCurrent($listed['sessions']));

        $current = $this->sessionById($listed['sessions'], $first['family_id']);
        self::assertTrue($current['current']);
        self::assertSame('web', $current['client']);

        $other = $this->sessionById($listed['sessions'], $second['family_id']);
        self::assertFalse($other['current']);
    }

    public function testListedTimesAreIsoUtcStrings(): void
    {
        $session = $this->registerAndLogin();

        $listed = $this->auth->listSessions($session['user']['id'], $session['family_id']);

        $item = $listed['sessions'][0];
        self::assertMatchesRegularExpression('/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00$/', $item['created_at']);
        self::assertMatchesRegularExpression('/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00$/', $item['last_used_at']);
    }

    public function testCreatedAtIsTheFamilyExpiryLessTheAbsoluteLifetime(): void
    {
        $session = $this->registerAndLogin();

        $listed = $this->auth->listSessions($session['user']['id'], $session['family_id']);

        $expiry = $this->pdo()->prepare('SELECT family_expires_at FROM refresh_tokens WHERE family_id = ? LIMIT 1');
        $expiry->execute([$session['family_id']]);
        $familyExpiry = new DateTimeImmutable((string) $expiry->fetchColumn(), new DateTimeZone('UTC'));

        $expected = $familyExpiry->sub(new DateInterval('PT' . RefreshTokens::ABSOLUTE_SECONDS . 'S'));
        self::assertSame($expected->format(DATE_ATOM), $listed['sessions'][0]['created_at']);
    }

    public function testRevokingAnotherSessionEndsItsRefreshToken(): void
    {
        $current = $this->registerAndLogin();
        $other = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $this->auth->revokeSession($current['user']['id'], $other['family_id'], $current['family_id'], self::IP);

        $this->assertCode('refresh_invalid', fn () => $this->auth->refresh($other['refresh_token'], 'web', self::IP));
        $kept = $this->auth->refresh($current['refresh_token'], 'web', self::IP);
        self::assertSame($current['family_id'], $kept['family_id']);
    }

    public function testRevokingTheCurrentSessionIsRefused(): void
    {
        $current = $this->registerAndLogin();
        $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $this->assertCode('cannot_revoke_current', fn () => $this->auth->revokeSession(
            $current['user']['id'], $current['family_id'], $current['family_id'], self::IP,
        ));
        $this->auth->refresh($current['refresh_token'], 'web', self::IP);
    }

    public function testMadeUpFamilyIdIsNotFound(): void
    {
        $current = $this->registerAndLogin();

        $this->assertCode('not_found', fn () => $this->auth->revokeSession(
            $current['user']['id'], self::UNKNOWN_FAMILY, $current['family_id'], self::IP,
        ));
    }

    public function testMalformedFamilyIdIsNotFound(): void
    {
        $current = $this->registerAndLogin();

        $this->assertCode('not_found', fn () => $this->auth->revokeSession(
            $current['user']['id'], 'not-a-family-id', $current['family_id'], self::IP,
        ));
        $this->assertCode('not_found', fn () => $this->auth->revokeSession(
            $current['user']['id'], strtoupper(self::UNKNOWN_FAMILY), $current['family_id'], self::IP,
        ));
    }

    public function testAnotherUsersFamilyIdIsNotFoundAndTheirSessionSurvives(): void
    {
        $current = $this->registerAndLogin();
        $this->registerAndVerify(self::OTHER_EMAIL);
        $theirs = $this->auth->login(self::OTHER_EMAIL, self::PASSWORD, self::IP);

        $this->assertCode('not_found', fn () => $this->auth->revokeSession(
            $current['user']['id'], $theirs['family_id'], $current['family_id'], self::IP,
        ));

        $kept = $this->auth->refresh($theirs['refresh_token'], 'web', self::IP);
        self::assertSame($theirs['family_id'], $kept['family_id']);
    }

    public function testRevokeOthersReturnsTheCountAndKeepsTheCurrentSession(): void
    {
        $current = $this->registerAndLogin();
        $second = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);
        $third = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $count = $this->auth->revokeOtherSessions($current['user']['id'], $current['family_id'], self::IP);

        self::assertSame(2, $count);
        $this->assertCode('refresh_invalid', fn () => $this->auth->refresh($second['refresh_token'], 'web', self::IP));
        $this->assertCode('refresh_invalid', fn () => $this->auth->refresh($third['refresh_token'], 'web', self::IP));

        $kept = $this->auth->refresh($current['refresh_token'], 'web', self::IP);
        self::assertSame($current['family_id'], $kept['family_id']);

        $listed = $this->auth->listSessions($current['user']['id'], $current['family_id']);
        self::assertCount(1, $listed['sessions']);
        self::assertTrue($listed['sessions'][0]['current']);
    }

    private function service(Mailer $mailer): AuthService
    {
        return new AuthService(
            new UserRepository(),
            new RefreshTokens(),
            new EmailTokens(),
            new RateLimiter(),
            new AuditLog(),
            new PasswordHasher(),
            new PasswordPolicy(new NullBreachChecker()),
            $this->jwt(),
            $mailer,
            'http://localhost:3000',
        );
    }

    private function jwt(): JwtIssuer
    {
        return new JwtIssuer(self::SECRET, 'https://issuer.test');
    }

    /** @return array{user: array<string, mixed>, access_token: string, expires_in: int, refresh_token: string, family_id: string} */
    private function registerAndLogin(): array
    {
        $this->registerAndVerify(self::EMAIL);

        return $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);
    }

    private function registerAndVerify(string $email): void
    {
        $this->auth->register($email, self::PASSWORD, null, self::IP);
        $this->auth->verifyEmail($this->tokenFromLastMail($email), self::IP);
    }

    private function tokenFromLastMail(string $email): string
    {
        $sent = array_values(array_filter(
            $this->mailer->sent(),
            static fn (array $message): bool => $message['to'] === $email,
        ));
        self::assertNotEmpty($sent, 'Expected an account mail.');

        preg_match('/token=([a-f0-9]{64})/', $sent[array_key_last($sent)]['text'], $matches);
        self::assertArrayHasKey(1, $matches, 'The mail carries no token.');

        return $matches[1];
    }

    /**
     * @param list<array<string, mixed>> $sessions
     */
    private function countCurrent(array $sessions): int
    {
        return count(array_filter($sessions, static fn (array $session): bool => $session['current'] === true));
    }

    /**
     * @param list<array<string, mixed>> $sessions
     * @return array<string, mixed>
     */
    private function sessionById(array $sessions, string $familyId): array
    {
        foreach ($sessions as $session) {
            if ($session['id'] === $familyId) {
                return $session;
            }
        }

        self::fail('No session has the family id.');
    }

    private function catchAuth(callable $call): AuthException
    {
        try {
            $call();
        } catch (AuthException $error) {
            return $error;
        }

        self::fail('Expected an AuthException.');
    }

    private function assertCode(string $code, callable $call): void
    {
        self::assertSame($code, $this->catchAuth($call)->errorCode);
    }
}
