<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\AuditLog;
use app\Auth\AuthService;
use app\Auth\EmailTokens;
use app\Auth\JwtIssuer;
use app\Auth\NullBreachChecker;
use app\Auth\OAuth\GoogleProvider;
use app\Auth\OAuth\OAuthException;
use app\Auth\OAuth\OAuthService;
use app\Auth\OAuth\OAuthStates;
use app\Auth\PasswordHasher;
use app\Auth\PasswordPolicy;
use app\Auth\RateLimiter;
use app\Auth\RefreshTokens;
use app\Config\Config;
use app\Mail\MemoryMailer;
use app\Users\UserRepository;
use tests\Support\DatabaseTestCase;

final class OAuthServiceTest extends DatabaseTestCase
{
    private const APP_URL = 'http://localhost:3000';
    private const IP = '198.51.100.9';
    private const EMAIL = 'pilot@example.test';
    private const SUBJECT = 'google-subject-001';
    private const CLIENT_ID = 'test-google-client-id';
    private const JWT_SECRET = 'OAUTH_SERVICE_TEST_SECRET_0123456789';
    private const GOOGLE_JWKS = 'https://www.googleapis.com/oauth2/v3/certs';
    private const GOOGLE_TOKEN = 'https://oauth2.googleapis.com/token';

    private SigningKey $key;
    private FakeTransport $transport;
    private UserRepository $users;
    private OAuthService $oauth;

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('GOOGLE_CLIENT_ID', self::CLIENT_ID);
        Config::set('GOOGLE_CLIENT_SECRET', 'test-google-client-secret');
        $this->pdo()->exec('TRUNCATE TABLE oauth_states');
        (new MemoryMailer())->reset();

        $this->key = SigningKey::generate('google-test-key');
        $this->transport = new FakeTransport([
            self::GOOGLE_JWKS => FakeTransport::json($this->key->jwks()),
        ]);
        $this->users = new UserRepository();

        $this->oauth = new OAuthService(
            $this->authService(),
            $this->users,
            new OAuthStates(),
            $this->transport,
            self::APP_URL,
        );
    }

    protected function tearDown(): void
    {
        Config::reset();

        parent::tearDown();
    }

    public function testANewVerifiedEmailCreatesAVerifiedUserWithoutAPasswordAndReturnsASession(): void
    {
        $started = $this->startSignIn();

        $result = $this->completeSignIn($started, ['name' => 'Pilot']);

        $user = $this->users->findByEmail(self::EMAIL);
        self::assertNotNull($user);
        self::assertNotNull($user['email_verified_at']);
        self::assertNull($user['password_hash']);
        self::assertSame('Pilot', $user['display_name']);

        self::assertNotEmpty($result['session']['access_token']);
        self::assertNotEmpty($result['session']['refresh_token']);
        self::assertSame((int) $user['id'], $result['session']['user']['id']);
        self::assertSame('/account', $result['next']);

        $identity = $this->pdo()->query('SELECT user_id, provider, provider_user_id, email_verified FROM user_identities')
            ->fetch();
        self::assertSame((int) $user['id'], (int) $identity['user_id']);
        self::assertSame('google', $identity['provider']);
        self::assertSame(self::SUBJECT, $identity['provider_user_id']);
        self::assertSame(1, (int) $identity['email_verified']);
    }

    public function testAVerifiedLocalAccountWithTheSameEmailIsLinked(): void
    {
        $existingId = $this->users->create(self::EMAIL, null, null);
        $this->users->markVerified($existingId);

        $result = $this->completeSignIn($this->startSignIn());

        self::assertSame($existingId, $result['session']['user']['id']);
        self::assertSame(1, $this->rowCount('users'));
        self::assertSame($existingId, $this->identityUserId());
    }

    public function testAnUnverifiedLocalAccountIsNotLinked(): void
    {
        $this->users->create(self::EMAIL, null, null);

        $this->assertOAuthError(
            'oauth_account_unverified',
            fn () => $this->completeSignIn($this->startSignIn()),
        );

        self::assertSame(0, $this->rowCount('user_identities'));
        self::assertNull($this->users->findByEmail(self::EMAIL)['email_verified_at']);
    }

    public function testAnUnverifiedProviderEmailIsRejected(): void
    {
        $this->assertOAuthError(
            'oauth_identity',
            fn () => $this->completeSignIn($this->startSignIn(), ['email_verified' => false]),
        );

        self::assertNull($this->users->findByEmail(self::EMAIL));
        self::assertSame(0, $this->rowCount('user_identities'));
    }

    public function testAVerifiedEmailOfADeletedAccountIsRejected(): void
    {
        $userId = $this->users->create(self::EMAIL, null, null);
        $this->users->markVerified($userId);
        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$userId]);

        $this->assertOAuthError('oauth_identity', fn () => $this->completeSignIn($this->startSignIn()));

        self::assertSame(0, $this->rowCount('user_identities'));
    }

    public function testAProviderEmailWithMixedCaseAndSpacesIsStoredNormalizedForANewAccount(): void
    {
        $this->completeSignIn($this->startSignIn(), ['email' => '  Pilot.Mixed@Example.TEST ']);

        self::assertNotNull($this->users->findByEmail('pilot.mixed@example.test'));
        self::assertSame(
            'pilot.mixed@example.test',
            $this->pdo()->query('SELECT email FROM user_identities')->fetchColumn(),
        );
    }

    public function testANonceThatDoesNotMatchTheStartedSignInIsRejected(): void
    {
        $this->assertOAuthError(
            'oauth_identity',
            fn () => $this->completeSignIn($this->startSignIn(), ['nonce' => 'replayed-nonce']),
        );

        self::assertNull($this->users->findByEmail(self::EMAIL));
    }

    public function testAStateUsedTwiceIsRejected(): void
    {
        $started = $this->startSignIn();
        $this->completeSignIn($started);

        $this->assertOAuthError('oauth_state', fn () => $this->completeSignIn($started));
    }

    public function testAnExistingIdentitySignsInDirectly(): void
    {
        $first = $this->completeSignIn($this->startSignIn());
        $userId = $first['session']['user']['id'];

        // The address changed at the provider. The subject still names the same account.
        $second = $this->completeSignIn($this->startSignIn(), ['email' => 'renamed@example.test']);

        self::assertSame($userId, $second['session']['user']['id']);
        self::assertSame(1, $this->rowCount('users'));
        self::assertSame(1, $this->rowCount('user_identities'));
        self::assertSame(2, $this->rowCount('audit_events', "type = 'login_sso'"));
    }

    public function testTheTokenRequestSendsTheVerifierThatMatchesTheChallenge(): void
    {
        $started = $this->startSignIn();
        $this->completeSignIn($started);

        $tokenRequests = array_values(array_filter(
            $this->transport->requests(),
            static fn (array $request): bool => $request['url'] === self::GOOGLE_TOKEN,
        ));
        self::assertCount(1, $tokenRequests);

        $verifier = $tokenRequests[0]['form']['code_verifier'];
        $challenge = rtrim(strtr(base64_encode(hash('sha256', $verifier, true)), '+/', '-_'), '=');

        self::assertSame($started['code_challenge'], $challenge);
        self::assertSame(self::APP_URL . '/api/auth/google/callback', $tokenRequests[0]['form']['redirect_uri']);
    }

    /**
     * Starts a sign-in through the real provider and returns the values the provider would echo back.
     *
     * @return array{state: string, nonce: string, code_challenge: string}
     */
    private function startSignIn(string $next = '/account'): array
    {
        $started = $this->oauth->start(new GoogleProvider(), $next);

        parse_str((string) parse_url($started['url'], PHP_URL_QUERY), $query);

        return [
            'state' => (string) $query['state'],
            'nonce' => (string) $query['nonce'],
            'code_challenge' => (string) $query['code_challenge'],
        ];
    }

    /**
     * Completes the callback. The token response carries an ID token signed with the test key, bound to the started nonce.
     *
     * @param array{state: string, nonce: string, code_challenge: string} $started
     * @param array<string, mixed> $claims Overrides for the ID token claims.
     * @return array{session: array<string, mixed>, next: string}
     */
    private function completeSignIn(array $started, array $claims = []): array
    {
        $idToken = $this->key->sign(array_merge([
            'iss' => 'https://accounts.google.com',
            'aud' => self::CLIENT_ID,
            'sub' => self::SUBJECT,
            'email' => self::EMAIL,
            'email_verified' => true,
            'name' => 'Pilot',
            'iat' => time(),
            'exp' => time() + 300,
            'nonce' => $started['nonce'],
        ], $claims));

        $this->transport->respondWith(self::GOOGLE_TOKEN, FakeTransport::json([
            'access_token' => 'provider-access-token',
            'id_token' => $idToken,
        ]));

        return $this->oauth->callback(new GoogleProvider(), 'provider-code', $started['state'], self::IP);
    }

    private function assertOAuthError(string $code, callable $call): void
    {
        try {
            $call();
        } catch (OAuthException $error) {
            self::assertSame($code, $error->errorCode);

            return;
        }

        self::fail('Expected an OAuthException.');
    }

    private function identityUserId(): int
    {
        return (int) $this->pdo()->query('SELECT user_id FROM user_identities LIMIT 1')->fetchColumn();
    }

    private function rowCount(string $table, string $where = '1 = 1'): int
    {
        return (int) $this->pdo()->query("SELECT COUNT(*) FROM $table WHERE $where")->fetchColumn();
    }

    private function authService(): AuthService
    {
        return new AuthService(
            new UserRepository(),
            new RefreshTokens(),
            new EmailTokens(),
            new RateLimiter(),
            new AuditLog(),
            new PasswordHasher(),
            new PasswordPolicy(new NullBreachChecker()),
            new JwtIssuer(self::JWT_SECRET, 'https://issuer.test'),
            new MemoryMailer(),
            self::APP_URL,
        );
    }
}
