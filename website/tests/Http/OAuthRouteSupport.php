<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\OAuth\Transport;
use app\Auth\OAuth\Transports;
use app\Config\Config;
use app\Http\Cookies;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use tests\Auth\OAuth\FakeTransport;

/**
 * Shared set-up for the single sign-on route tests: the app, the fake transport and the request helpers.
 */
trait OAuthRouteSupport
{
    private const IP = '198.51.100.40';
    private const PROVIDERS = ['google', 'microsoft', 'github'];
    private const CLEARED_STATE_COOKIE = Cookies::OAUTH_STATE . '=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax';

    private static ?App $app = null;
    private ?Transport $previousTransport = null;

    protected function setUpOAuthRoute(): void
    {
        $this->previousTransport = Transports::current();
        Transports::use(new FakeTransport());
    }

    protected function tearDownOAuthRoute(): void
    {
        if ($this->previousTransport !== null) {
            Transports::use($this->previousTransport);
        }
        Config::reset();
    }

    private function appUrl(): string
    {
        return rtrim(Config::need('APP_URL'), '/');
    }

    private function newestStoredNext(): string
    {
        return (string) $this->pdo()->query('SELECT next_path FROM oauth_states ORDER BY created_at DESC LIMIT 1')->fetchColumn();
    }

    /**
     * Enables the named providers with test values. Every other provider gets an empty client ID, which counts as unset.
     *
     * @param list<string> $enabled
     */
    private function configure(array $enabled): void
    {
        foreach (self::PROVIDERS as $name) {
            $prefix = strtoupper($name);
            $on = in_array($name, $enabled, true);

            Config::set($prefix . '_CLIENT_ID', $on ? 'test-' . $name . '-client' : '');
            Config::set($prefix . '_CLIENT_SECRET', $on ? 'test-' . $name . '-secret' : '');
        }
    }

    private function app(): App
    {
        if (self::$app === null) {
            self::$app = require dirname(__DIR__, 2) . '/api/bootstrap.php';
        }

        return self::$app;
    }

    /**
     * A query given as an array is set directly. An empty array leaves the query in the URL to be parsed.
     *
     * @param array<string, string> $query
     * @param array<string, string> $cookies
     */
    private function send(string $method, string $path, array $query = [], array $cookies = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())
            ->createServerRequest($method, $path, ['REMOTE_ADDR' => self::IP])
            ->withCookieParams($cookies);

        if ($query !== []) {
            $request = $request->withQueryParams($query);
        }

        return $this->app()->handle($request);
    }

    /** @return array<string, mixed> */
    private function body(ResponseInterface $response): array
    {
        $decoded = json_decode((string) $response->getBody(), true, 512, JSON_THROW_ON_ERROR);
        $this->assertIsArray($decoded);

        return $decoded;
    }

    private function assertError(ResponseInterface $response, int $status, string $code): void
    {
        $this->assertSame($status, $response->getStatusCode());
        $this->assertSame(['error' => $code], $this->body($response));
    }

    /** @return array<string, string> The query string of a redirect, decoded. */
    private function redirectQuery(ResponseInterface $response): array
    {
        parse_str((string) parse_url($response->getHeaderLine('Location'), PHP_URL_QUERY), $query);

        return $query;
    }
}
