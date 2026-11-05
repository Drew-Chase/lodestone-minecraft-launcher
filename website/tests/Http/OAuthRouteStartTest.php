<?php

declare(strict_types=1);

namespace tests\Http;

use app\Http\Cookies;
use PHPUnit\Framework\Attributes\DataProvider;
use tests\Support\DatabaseTestCase;

/**
 * The start endpoint of the single sign-on routes: the redirect, the state cookie, the next path and the rate limit.
 * No test reaches a provider.
 */
final class OAuthRouteStartTest extends DatabaseTestCase
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

    /** @return iterable<string, array{string}> */
    public static function rejectedNextPaths(): iterable
    {
        yield 'protocol-relative' => ['//x'];
        yield 'one backslash' => ['/\\x'];
        yield 'tab' => ["/\tx"];
        yield 'line feed' => ["/\nx"];
        yield 'no leading slash' => ['x'];
        yield '256 bytes' => ['/' . str_repeat('a', 255)];
        yield 'invalid UTF-8' => ["/\xFF"];
    }

    /** @return iterable<string, array{string}> */
    public static function acceptedNextPaths(): iterable
    {
        yield 'encoded slashes' => ['/%2F%2Fx'];
        yield 'at sign' => ['/@x'];
        yield 'dashboard' => ['/dashboard'];
        yield 'exactly 255 bytes' => ['/' . str_repeat('a', 254)];
    }

    public function test_start_redirects_to_google_with_the_exact_pkce_challenge(): void
    {
        $this->configure(['google']);

        $response = $this->send('GET', '/api/auth/google/start', ['next' => '/account/security']);

        $this->assertSame(302, $response->getStatusCode());
        $location = $response->getHeaderLine('Location');
        $this->assertStringStartsWith('https://accounts.google.com/', $location);

        $query = $this->redirectQuery($response);
        $verifier = (string) $this->pdo()->query(
            'SELECT pkce_verifier FROM oauth_states ORDER BY created_at DESC LIMIT 1'
        )->fetchColumn();
        $expected = rtrim(strtr(base64_encode(hash('sha256', $verifier, true)), '+/', '-_'), '=');
        $this->assertSame($expected, $query['code_challenge']);
        $this->assertSame('S256', $query['code_challenge_method']);
        $this->assertSame($this->appUrl() . '/api/auth/google/callback', $query['redirect_uri']);
    }

    public function test_start_stores_an_unsafe_next_as_the_account_page(): void
    {
        $this->configure(['google']);

        $this->send('GET', '/api/auth/google/start', ['next' => '//evil.example/steal']);

        $this->assertSame('/account', $this->newestStoredNext());
    }

    #[DataProvider('rejectedNextPaths')]
    public function test_start_stores_a_rejected_next_as_the_account_page(string $next): void
    {
        $this->configure(['google']);

        $this->send('GET', '/api/auth/google/start?next=' . rawurlencode($next));

        $this->assertSame('/account', $this->newestStoredNext());
    }

    #[DataProvider('acceptedNextPaths')]
    public function test_start_stores_an_accepted_next_unchanged(string $next): void
    {
        $this->configure(['google']);

        $this->send('GET', '/api/auth/google/start?next=' . rawurlencode($next));

        $this->assertSame($next, $this->newestStoredNext());
    }

    public function test_start_stores_a_next_that_is_not_valid_utf8_as_the_account_page(): void
    {
        $this->configure(['google']);

        $this->send('GET', '/api/auth/google/start', ['next' => "/\xFF"]);

        $this->assertSame('/account', $this->newestStoredNext());
    }

    public function test_start_stores_a_next_with_a_control_character_as_the_account_page(): void
    {
        $this->configure(['google']);

        $this->send('GET', '/api/auth/google/start', ['next' => "/a\x01b"]);

        $this->assertSame('/account', $this->newestStoredNext());
    }

    public function test_start_sets_the_state_cookie_with_the_bound_attributes(): void
    {
        $this->configure(['google']);

        $response = $this->send('GET', '/api/auth/google/start', ['next' => '/account']);

        $state = $this->redirectQuery($response)['state'];
        $this->assertSame(
            [Cookies::OAUTH_STATE . '=' . $state . '; Max-Age=600; Path=/; HttpOnly; Secure; SameSite=Lax'],
            $response->getHeader('Set-Cookie'),
        );
    }

    public function test_start_for_an_unknown_provider_returns_404_not_found(): void
    {
        $this->configure(['google']);

        $response = $this->send('GET', '/api/auth/facebook/start');

        $this->assertError($response, 404, 'not_found');
    }

    public function test_start_for_a_disabled_provider_returns_404_not_found(): void
    {
        $this->configure([]);

        $response = $this->send('GET', '/api/auth/google/start');

        $this->assertError($response, 404, 'not_found');
    }

    public function test_the_31st_start_from_one_address_is_rate_limited(): void
    {
        $this->configure(['google']);

        for ($i = 0; $i < 30; $i++) {
            $this->assertSame(302, $this->send('GET', '/api/auth/google/start')->getStatusCode());
        }

        $response = $this->send('GET', '/api/auth/google/start');

        $this->assertError($response, 429, 'rate_limited');
        $this->assertMatchesRegularExpression('/^\d+$/', $response->getHeaderLine('Retry-After'));
        $this->assertGreaterThanOrEqual(1, (int) $response->getHeaderLine('Retry-After'));
        $this->assertLessThanOrEqual(900, (int) $response->getHeaderLine('Retry-After'));
    }
}
