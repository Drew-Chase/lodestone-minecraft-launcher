<?php

declare(strict_types=1);

namespace app\Http;

use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;

/**
 * The refresh cookie and the single sign-on state cookie. Both carry the __Host- prefix, which needs Secure, Path=/
 * and no Domain, so a sibling subdomain cannot plant a cookie with the same name. The refresh cookie is therefore
 * sent with every request to the site, and only the refresh and logout routes read it.
 */
final class Cookies
{
    public const REFRESH = '__Host-lodestone_refresh';
    public const OAUTH_STATE = '__Host-lodestone_oauth_state';

    private const REFRESH_MAX_AGE = 2592000;
    private const ATTRIBUTES = '; Path=/; HttpOnly; Secure; SameSite=Strict';

    private const OAUTH_STATE_MAX_AGE = 600;
    // The __Host- prefix makes the browser refuse this cookie unless it is Secure, has Path=/ and has no Domain. A sibling
    // subdomain therefore cannot plant a cookie with the same name.
    // Lax, not Strict: the provider sends the browser back to the callback with a cross-site top-level GET, and
    // Strict cookies are not sent on that request. Lax is still refused on every cross-site POST.
    private const OAUTH_STATE_ATTRIBUTES = '; Path=/; HttpOnly; Secure; SameSite=Lax';

    private function __construct()
    {
    }

    public static function setRefresh(ResponseInterface $response, string $token): ResponseInterface
    {
        return $response->withAddedHeader(
            'Set-Cookie',
            self::REFRESH . '=' . $token . '; Max-Age=' . self::REFRESH_MAX_AGE . self::ATTRIBUTES
        );
    }

    public static function clearRefresh(ResponseInterface $response): ResponseInterface
    {
        return $response->withAddedHeader('Set-Cookie', self::REFRESH . '=; Max-Age=0' . self::ATTRIBUTES);
    }

    /** Returns '' when the cookie is absent. */
    public static function readRefresh(ServerRequestInterface $request): string
    {
        $value = $request->getCookieParams()[self::REFRESH] ?? '';

        return is_string($value) ? $value : '';
    }

    /** Binds a sign-in state to the browser that started it, for ten minutes. */
    public static function setOAuthState(ResponseInterface $response, string $state): ResponseInterface
    {
        return $response->withAddedHeader(
            'Set-Cookie',
            self::OAUTH_STATE . '=' . $state . '; Max-Age=' . self::OAUTH_STATE_MAX_AGE . self::OAUTH_STATE_ATTRIBUTES
        );
    }

    public static function clearOAuthState(ResponseInterface $response): ResponseInterface
    {
        return $response->withAddedHeader('Set-Cookie', self::OAUTH_STATE . '=; Max-Age=0' . self::OAUTH_STATE_ATTRIBUTES);
    }

    /** Returns '' when the cookie is absent. */
    public static function readOAuthState(ServerRequestInterface $request): string
    {
        $value = $request->getCookieParams()[self::OAUTH_STATE] ?? '';

        return is_string($value) ? $value : '';
    }
}
