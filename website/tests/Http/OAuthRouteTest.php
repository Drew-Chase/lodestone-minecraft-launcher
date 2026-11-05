<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\OAuth\Transports;
use app\Config\Config;
use app\Http\Cookies;
use app\Users\UserRepository;
use tests\Auth\OAuth\FakeTransport;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of the single sign-on callback. No test reaches a provider: the transport is a fake.
 * The start endpoint is covered by OAuthRouteStartTest.
 */
final class OAuthRouteTest extends DatabaseTestCase
{
    use OAuthRouteSupport;

    protected function setUp(): void
    {
        parent::setUp();

        $this->pdo()->exec('TRUNCATE TABLE oauth_states');
        $this->setUpOAuthRoute();
    }

    protected function tearDown(): void
    {
        $this->tearDownOAuthRoute();

        parent::tearDown();
    }

    public function test_providers_lists_a_provider_whose_id_and_secret_are_set(): void
    {
        $this->configure(['google']);

        $response = $this->send('GET', '/api/auth/providers');

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['providers' => ['google']], $this->body($response));
    }

    public function test_providers_lists_nothing_when_the_client_id_is_empty(): void
    {
        $this->configure([]);
        Config::set('GOOGLE_CLIENT_ID', '');
        Config::set('GOOGLE_CLIENT_SECRET', 'test-google-secret');

        $response = $this->send('GET', '/api/auth/providers');

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['providers' => []], $this->body($response));
    }

    public function test_callback_for_an_unknown_provider_returns_404_not_found(): void
    {
        $this->configure(['google']);

        $response = $this->send(
            'GET',
            '/api/auth/facebook/callback',
            ['code' => 'provider-code', 'state' => 'some-state'],
            [Cookies::OAUTH_STATE => 'some-state'],
        );

        $this->assertError($response, 404, 'not_found');
    }

    public function test_callback_with_an_error_and_the_bound_state_redirects_as_denied(): void
    {
        $this->configure(['google']);

        $response = $this->send(
            'GET',
            '/api/auth/google/callback',
            ['error' => 'access_denied', 'state' => 'some-state'],
            [Cookies::OAUTH_STATE => 'some-state'],
        );

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/login?error=oauth_denied', $response->getHeaderLine('Location'));
    }

    public function test_callback_with_access_denied_and_the_bound_state_clears_the_state_cookie(): void
    {
        $this->configure(['google']);

        $response = $this->send(
            'GET',
            '/api/auth/google/callback',
            ['error' => 'access_denied', 'state' => 'some-state'],
            [Cookies::OAUTH_STATE => 'some-state'],
        );

        $this->assertSame([self::CLEARED_STATE_COOKIE], $response->getHeader('Set-Cookie'));
    }

    public function test_callback_with_access_denied_and_no_state_or_cookie_is_a_state_error_without_a_cookie_clear(): void
    {
        $this->configure(['google']);

        $response = $this->send('GET', '/api/auth/google/callback', ['error' => 'access_denied']);

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/login?error=oauth_state', $response->getHeaderLine('Location'));
        $this->assertSame([], $response->getHeader('Set-Cookie'));
    }

    public function test_callback_with_the_bound_state_and_no_code_redirects_as_a_token_error_and_clears_the_cookie(): void
    {
        $this->configure(['google']);

        $response = $this->send(
            'GET',
            '/api/auth/google/callback',
            ['state' => 'some-state'],
            [Cookies::OAUTH_STATE => 'some-state'],
        );

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/login?error=oauth_token', $response->getHeaderLine('Location'));
        $this->assertSame([self::CLEARED_STATE_COOKIE], $response->getHeader('Set-Cookie'));
    }

    public function test_callback_with_an_unknown_state_redirects_as_a_state_error(): void
    {
        $this->configure(['google']);

        $response = $this->send(
            'GET',
            '/api/auth/google/callback',
            ['code' => 'provider-code', 'state' => 'never-issued'],
            [Cookies::OAUTH_STATE => 'never-issued'],
        );

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/login?error=oauth_state', $response->getHeaderLine('Location'));
    }

    public function test_callback_without_a_state_cookie_is_a_state_error_without_a_cookie_clear(): void
    {
        $this->configure(['google']);

        $response = $this->send('GET', '/api/auth/google/callback', ['code' => 'provider-code', 'state' => 'some-state']);

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/login?error=oauth_state', $response->getHeaderLine('Location'));
        $this->assertNotContains(self::CLEARED_STATE_COOKIE, $response->getHeader('Set-Cookie'));
    }

    public function test_callback_whose_cookie_differs_from_the_query_state_is_a_state_error_without_a_cookie_clear(): void
    {
        $this->configure(['google']);

        $response = $this->send(
            'GET',
            '/api/auth/google/callback',
            ['code' => 'provider-code', 'state' => 'query-state'],
            [Cookies::OAUTH_STATE => 'cookie-state'],
        );

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/login?error=oauth_state', $response->getHeaderLine('Location'));
        $this->assertNotContains(self::CLEARED_STATE_COOKIE, $response->getHeader('Set-Cookie'));
    }

    public function test_a_github_sign_in_redirects_to_the_stored_next_path_and_sets_the_refresh_cookie(): void
    {
        $this->configure(['github']);
        Transports::use(new FakeTransport([
            'https://github.com/login/oauth/access_token' => FakeTransport::json(['access_token' => 'gh-access']),
            'https://api.github.com/user' => FakeTransport::json(['id' => 4242, 'name' => 'Pilot']),
            'https://api.github.com/user/emails' => FakeTransport::json([
                ['email' => ' Pilot.Mixed@Example.TEST ', 'primary' => true, 'verified' => true],
            ]),
        ]));

        $started = $this->send('GET', '/api/auth/github/start', ['next' => '/account/security']);
        $state = (string) $this->redirectQuery($started)['state'];

        $response = $this->send(
            'GET',
            '/api/auth/github/callback',
            ['code' => 'provider-code', 'state' => $state],
            [Cookies::OAUTH_STATE => $state],
        );

        $this->assertSame(302, $response->getStatusCode());
        $this->assertSame($this->appUrl() . '/account/security', $response->getHeaderLine('Location'));

        $cookies = $response->getHeader('Set-Cookie');
        $this->assertCount(2, $cookies);
        $this->assertMatchesRegularExpression(
            '/^__Host-lodestone_refresh=[^;]+; Max-Age=2592000; Path=\/; HttpOnly; Secure; SameSite=Strict$/',
            $cookies[0],
        );
        $this->assertSame(self::CLEARED_STATE_COOKIE, $cookies[1]);

        $user = (new UserRepository())->findByEmail('pilot.mixed@example.test');
        $this->assertNotNull($user);
        $this->assertSame('pilot.mixed@example.test', $user['email']);

        $statement = $this->pdo()->prepare('SELECT provider, provider_user_id FROM user_identities WHERE user_id = ?');
        $statement->execute([(int) $user['id']]);
        $this->assertSame(
            ['provider' => 'github', 'provider_user_id' => '4242'],
            $statement->fetch(\PDO::FETCH_ASSOC),
        );
    }
}
