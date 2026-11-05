<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use app\Auth\AuthException;
use app\Auth\AuthService;
use app\Db\Database;
use app\Users\UserRepository;

/**
 * Starts and completes a single sign-on. Every failure is an OAuthException with the code from the README.
 * Nothing here knows about HTTP.
 */
final class OAuthService
{
    public function __construct(
        private readonly AuthService $auth,
        private readonly UserRepository $users,
        private readonly OAuthStates $states,
        private readonly Transport $transport,
        private readonly string $appUrl,
    ) {
    }

    /**
     * Creates the single-use state and returns the provider's sign-in URL with the state it carries.
     * The route binds the state to the browser with a cookie.
     *
     * @return array{url: string, state: string}
     */
    public function start(Provider $provider, string $nextPath): array
    {
        $started = $this->states->create($provider->name(), $nextPath);

        return [
            'url' => $provider->authorizeUrl(
                $started['state'],
                $started['code_challenge'],
                $started['nonce'],
                $this->redirectUri($provider),
            ),
            'state' => $started['state'],
        ];
    }

    /**
     * @return array{session: array<string, mixed>, next: string}
     * @throws OAuthException oauth_state, oauth_token, oauth_identity or oauth_account_unverified
     */
    public function callback(Provider $provider, string $code, string $state, string $ip): array
    {
        $stored = $this->states->consume($state, $provider->name());
        if ($stored === null) {
            throw new OAuthException('oauth_state');
        }

        try {
            $tokens = $provider->exchange($code, $stored['pkce_verifier'], $this->redirectUri($provider), $this->transport);
        } catch (\RuntimeException) {
            throw new OAuthException('oauth_token');
        }

        try {
            $identity = $provider->identity($tokens, $stored['nonce'], $this->transport);
        } catch (UpstreamFailure) {
            // The provider's endpoint failed, so the person has no identity to be refused for.
            throw new OAuthException('oauth_token');
        } catch (\RuntimeException) {
            throw new OAuthException('oauth_identity');
        }

        $userId = $this->userFor($identity);

        try {
            $session = $this->auth->startSession($userId, $ip);
        } catch (AuthException) {
            // The account is missing or deleted, so the identity cannot sign anyone in.
            throw new OAuthException('oauth_identity');
        }

        return ['session' => $session, 'next' => $stored['next_path']];
    }

    /**
     * Finds or makes the local account for a provider identity. Links only to a verified account.
     *
     * @throws OAuthException oauth_identity or oauth_account_unverified
     */
    private function userFor(ProviderIdentity $identity): int
    {
        $linked = $this->linkedUserId($identity->provider, $identity->subject);
        if ($linked !== null) {
            return $linked;
        }

        if (!$identity->emailVerified) {
            throw new OAuthException('oauth_identity');
        }

        try {
            $email = $this->auth->normalizeEmail($identity->email);
        } catch (AuthException) {
            throw new OAuthException('oauth_identity');
        }

        $existing = $this->users->findByEmail($email);
        if ($existing !== null) {
            if ($existing['deleted_at'] !== null) {
                throw new OAuthException('oauth_identity');
            }

            // An unverified account may hold a password chosen by someone else, so it is never linked.
            if ($existing['email_verified_at'] === null) {
                throw new OAuthException('oauth_account_unverified');
            }

            $userId = (int) $existing['id'];
            $this->insertIdentity($userId, $identity, $email);

            return $userId;
        }

        // The account, its verification and its identity are one unit: a failure leaves none of them behind.
        return Database::transaction(function () use ($email, $identity): int {
            $userId = $this->users->create($email, null, $this->displayName($identity->name));
            $this->users->markVerified($userId);
            $this->insertIdentity($userId, $identity, $email);

            return $userId;
        });
    }

    private function linkedUserId(string $provider, string $subject): ?int
    {
        $statement = Database::pdo()->prepare(
            'SELECT user_id FROM user_identities WHERE provider = ? AND provider_user_id = ?'
        );
        $statement->execute([$provider, $subject]);
        $userId = $statement->fetchColumn();

        return $userId === false ? null : (int) $userId;
    }

    private function insertIdentity(int $userId, ProviderIdentity $identity, string $email): void
    {
        $statement = Database::pdo()->prepare(
            'INSERT INTO user_identities (user_id, provider, provider_user_id, email, email_verified)
             VALUES (?, ?, ?, ?, 1)'
        );
        $statement->execute([$userId, $identity->provider, $identity->subject, $email]);
    }

    /** A name the account rules reject is dropped rather than failing the sign-in. */
    private function displayName(?string $name): ?string
    {
        try {
            return $this->auth->validDisplayName($name);
        } catch (AuthException) {
            return null;
        }
    }

    private function redirectUri(Provider $provider): string
    {
        return rtrim($this->appUrl, '/') . '/api/auth/' . $provider->name() . '/callback';
    }
}
