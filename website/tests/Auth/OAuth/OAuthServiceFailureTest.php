<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\AuditLog;
use app\Auth\AuthService;
use app\Auth\EmailTokens;
use app\Auth\JwtIssuer;
use app\Auth\NullBreachChecker;
use app\Auth\OAuth\GitHubProvider;
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

/**
 * Provider failures during a callback: a failed code exchange, a failed identity fetch, and an account with no usable email.
 */
final class OAuthServiceFailureTest extends DatabaseTestCase
{
    private const APP_URL = 'http://localhost:3000';
    private const IP = '198.51.100.11';
    private const JWT_SECRET = 'OAUTH_SERVICE_FAILURE_SECRET_0123456789';
    private const GOOGLE_TOKEN = 'https://oauth2.googleapis.com/token';
    private const GOOGLE_JWKS = 'https://www.googleapis.com/oauth2/v3/certs';
    private const GITHUB_TOKEN = 'https://github.com/login/oauth/access_token';
    private const GITHUB_USER = 'https://api.github.com/user';
    private const GITHUB_EMAILS = 'https://api.github.com/user/emails';

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('GOOGLE_CLIENT_ID', 'test-google-client-id');
        Config::set('GOOGLE_CLIENT_SECRET', 'test-google-client-secret');
        Config::set('GITHUB_CLIENT_ID', 'test-github-client-id');
        Config::set('GITHUB_CLIENT_SECRET', 'test-github-client-secret');
        (new MemoryMailer())->reset();
    }

    protected function tearDown(): void
    {
        Config::reset();

        parent::tearDown();
    }

    public function testAFailedCodeExchangeStillSpendsTheState(): void
    {
        $started = (new OAuthStates())->create('google', '/account');
        $transport = new FakeTransport([self::GOOGLE_TOKEN => FakeTransport::json(['error' => 'invalid_grant'], 400)]);
        $service = $this->service($transport);

        $this->assertOAuthError('oauth_token', fn () => $service->callback(
            new GoogleProvider(),
            'stale-code',
            $started['state'],
            self::IP,
        ));

        $this->assertOAuthError('oauth_state', fn () => $service->callback(
            new GoogleProvider(),
            'stale-code',
            $started['state'],
            self::IP,
        ));
    }

    public function testAFailedGitHubIdentityFetchIsATokenErrorNotAnIdentityError(): void
    {
        $started = (new OAuthStates())->create('github', '/account');
        $transport = new FakeTransport([
            self::GITHUB_TOKEN => FakeTransport::json(['access_token' => 'gh-access']),
            self::GITHUB_USER => FakeTransport::json([], 500),
        ]);

        $this->assertOAuthError('oauth_token', fn () => $this->service($transport)->callback(
            new GitHubProvider(),
            'provider-code',
            $started['state'],
            self::IP,
        ));

        self::assertSame(0, $this->rowCount('users'));
    }

    public function testAGitHubAccountWithNoVerifiedPrimaryEmailIsAnIdentityError(): void
    {
        $started = (new OAuthStates())->create('github', '/account');
        $transport = new FakeTransport([
            self::GITHUB_TOKEN => FakeTransport::json(['access_token' => 'gh-access']),
            self::GITHUB_USER => FakeTransport::json(['id' => 4242, 'name' => 'Pilot']),
            self::GITHUB_EMAILS => FakeTransport::json([
                ['email' => 'pilot@example.test', 'primary' => true, 'verified' => false],
            ]),
        ]);

        $this->assertOAuthError('oauth_identity', fn () => $this->service($transport)->callback(
            new GitHubProvider(),
            'provider-code',
            $started['state'],
            self::IP,
        ));

        self::assertSame(0, $this->rowCount('users'));
        self::assertSame(0, $this->rowCount('user_identities'));
    }

    public function testAFailedGoogleKeySetFetchIsATokenErrorNotAnIdentityError(): void
    {
        $started = (new OAuthStates())->create('google', '/account');
        $transport = new FakeTransport([
            self::GOOGLE_TOKEN => FakeTransport::json(['id_token' => 'header.payload.signature']),
            self::GOOGLE_JWKS => FakeTransport::json([], 500),
        ]);

        $this->assertOAuthError('oauth_token', fn () => $this->service($transport)->callback(
            new GoogleProvider(),
            'provider-code',
            $started['state'],
            self::IP,
        ));

        self::assertSame(0, $this->rowCount('users'));
    }

    private function service(FakeTransport $transport): OAuthService
    {
        return new OAuthService(
            $this->authService(),
            new UserRepository(),
            new OAuthStates(),
            $transport,
            self::APP_URL,
        );
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

    private function rowCount(string $table): int
    {
        return (int) $this->pdo()->query("SELECT COUNT(*) FROM $table")->fetchColumn();
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
