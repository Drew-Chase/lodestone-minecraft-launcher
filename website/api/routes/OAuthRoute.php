<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Auth\AuthService;
use app\Auth\OAuth\OAuthException;
use app\Auth\OAuth\OAuthService;
use app\Auth\OAuth\OAuthStates;
use app\Auth\OAuth\Provider;
use app\Auth\OAuth\Providers;
use app\Auth\OAuth\Transports;
use app\Auth\RateLimiter;
use app\Config\Config;
use app\Http\ClientIp;
use app\Http\Cookies;
use app\Http\Json;
use app\Users\UserRepository;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * The single sign-on routes under /api/auth. A sign-in ends with a redirect to the website, never a JSON body.
 * The state from start is bound to the browser that started it, so a callback completed in another browser is refused.
 */
final class OAuthRoute
{
    private const DEFAULT_NEXT = '/account';
    private const NEXT_MAX_LENGTH = 255;
    private const LOGIN_PATH = '/login';
    private const START_LIMIT = 30;
    private const START_WINDOW_SECONDS = 900;

    public static function configure(App $app): void
    {
        $app->get('/auth/providers', [self::class, 'providers']);
        $app->get('/auth/{provider}/start', [self::class, 'start']);
        $app->get('/auth/{provider}/callback', [self::class, 'callback']);
    }

    /** GET /api/auth/providers */
    public static function providers(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        return Json::respond($response, ['providers' => Providers::enabled()]);
    }

    /** GET /api/auth/{provider}/start?next=/path */
    public static function start(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        $provider = self::enabledProvider($args);
        if ($provider === null) {
            return Json::error($response, 'not_found', 404);
        }

        $limiter = new RateLimiter();
        $bucket = 'oauth:start:' . ClientIp::from($request);
        if ($limiter->hit($bucket, self::START_WINDOW_SECONDS) > self::START_LIMIT) {
            throw new AuthException('rate_limited', 429, $limiter->retryAfter(self::START_WINDOW_SECONDS));
        }

        $query = $request->getQueryParams();
        $next = self::safeNext($query['next'] ?? null);

        $started = self::services()->start($provider, $next);

        return Cookies::setOAuthState(self::redirect($response, $started['url']), $started['state']);
    }

    /** GET /api/auth/{provider}/callback?code=…&state=… */
    public static function callback(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        $provider = self::enabledProvider($args);
        if ($provider === null) {
            return Json::error($response, 'not_found', 404);
        }

        $query = $request->getQueryParams();
        $login = self::appUrl() . self::LOGIN_PATH;

        // The state must be the one this browser was given at start. That is checked before the provider's error is
        // read. A link without the bound state is refused and the cookie is left alone, so a link cannot cancel a
        // sign-in that is in progress.
        $state = $query['state'] ?? null;
        if (!is_string($state) || $state === '' || !self::matchesBoundState($request, $state)) {
            return self::redirect($response, $login . '?error=oauth_state');
        }

        if (isset($query['error'])) {
            return self::finish($response, $login . '?error=oauth_denied');
        }

        $code = $query['code'] ?? null;
        if (!is_string($code) || $code === '') {
            return self::finish($response, $login . '?error=oauth_token');
        }

        try {
            $result = self::services()->callback($provider, $code, $state, ClientIp::from($request));
        } catch (OAuthException $error) {
            return self::finish($response, $login . '?error=' . rawurlencode($error->errorCode));
        } catch (\Throwable $error) {
            // Only the class is logged. The message and the request can carry tokens or addresses.
            error_log('OAuth callback failed: ' . $error::class);

            return self::finish($response, $login . '?error=oauth_token');
        }

        $response = Cookies::setRefresh($response, $result['session']['refresh_token']);

        return self::finish($response, self::appUrl() . $result['next']);
    }

    /**
     * The provider named in the route, when it is configured. Anything else is null.
     *
     * @param array<string, mixed> $args
     */
    private static function enabledProvider(array $args): ?Provider
    {
        $name = $args['provider'] ?? null;
        if (!is_string($name) || !in_array($name, Providers::enabled(), true)) {
            return null;
        }

        return Providers::make($name);
    }

    /**
     * Only a path on this site is accepted: a single leading slash, no backslash, no control characters, valid UTF-8.
     * Anything else becomes the account page.
     */
    private static function safeNext(mixed $value): string
    {
        if (!is_string($value)
            || $value === ''
            || $value[0] !== '/'
            || str_starts_with($value, '//')
            || str_contains($value, '\\')
            || strlen($value) > self::NEXT_MAX_LENGTH
            || preg_match('/[\x00-\x1F\x7F]/', $value) === 1
            || !mb_check_encoding($value, 'UTF-8')
        ) {
            return self::DEFAULT_NEXT;
        }

        return $value;
    }

    /** The state in the query must equal the one this browser was given at start. A missing cookie never matches. */
    private static function matchesBoundState(ServerRequestInterface $request, string $state): bool
    {
        $bound = Cookies::readOAuthState($request);

        return $bound !== '' && hash_equals($bound, $state);
    }

    /** Ends a callback. The state cookie is spent whatever the outcome, and the browser is sent on. */
    private static function finish(ResponseInterface $response, string $url): ResponseInterface
    {
        return self::redirect(Cookies::clearOAuthState($response), $url);
    }

    private static function redirect(ResponseInterface $response, string $url): ResponseInterface
    {
        return $response->withStatus(302)->withHeader('Location', $url);
    }

    private static function appUrl(): string
    {
        return rtrim(Config::need('APP_URL'), '/');
    }

    /** The services for one request. Each handler builds its own, so tests can swap the transport before a request. */
    private static function services(): OAuthService
    {
        return new OAuthService(
            AuthService::fromConfig(),
            new UserRepository(),
            new OAuthStates(),
            Transports::current(),
            self::appUrl(),
        );
    }
}
