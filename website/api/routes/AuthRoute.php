<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Auth\AuthService;
use app\Http\Body;
use app\Http\ClientIp;
use app\Http\Cookies;
use app\Http\Json;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * The public /auth routes. The change-password route is in AccountRoute, since it needs a bearer token.
 */
final class AuthRoute
{
    private const CLIENT = 'web';
    private const LAUNCHER = 'launcher';

    public static function configure(App $app): void
    {
        $app->post('/auth/register', [self::class, 'register']);
        $app->post('/auth/verify-email', [self::class, 'verifyEmail']);
        $app->post('/auth/resend-verification', [self::class, 'resendVerification']);
        $app->post('/auth/login', [self::class, 'login']);
        $app->post('/auth/refresh', [self::class, 'refresh']);
        $app->post('/auth/logout', [self::class, 'logout']);
        $app->post('/auth/forgot-password', [self::class, 'forgotPassword']);
        $app->post('/auth/reset-password', [self::class, 'resetPassword']);
    }

    /** POST /api/auth/register */
    public static function register(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $body = Body::parse($request);

        $email = $body['email'] ?? null;
        if (!is_string($email)) {
            throw new AuthException('invalid_email', 400);
        }

        $password = $body['password'] ?? null;
        if (!is_string($password)) {
            throw new AuthException('missing_field', 400);
        }

        $name = $body['display_name'] ?? null;
        if ($name !== null && !is_string($name)) {
            throw new AuthException('invalid_display_name', 400);
        }

        AuthService::fromConfig()->register($email, $password, $name, ClientIp::from($request));

        return Json::respond($response, ['status' => 'check_email'], 202);
    }

    /** POST /api/auth/verify-email */
    public static function verifyEmail(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $body = Body::parse($request);

        $token = $body['token'] ?? null;
        if (!is_string($token)) {
            throw new AuthException('invalid_token', 400);
        }

        AuthService::fromConfig()->verifyEmail($token, ClientIp::from($request));

        return Json::respond($response, ['status' => 'verified']);
    }

    /** POST /api/auth/resend-verification */
    public static function resendVerification(
        ServerRequestInterface $request,
        ResponseInterface $response,
    ): ResponseInterface {
        $body = Body::parse($request);

        AuthService::fromConfig()->resendVerification(self::text($body, 'email'), ClientIp::from($request));

        return Json::respond($response, ['status' => 'check_email'], 202);
    }

    /** POST /api/auth/login */
    public static function login(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $body = Body::parse($request);

        $session = AuthService::fromConfig()->login(
            self::text($body, 'email'),
            self::text($body, 'password'),
            ClientIp::from($request),
        );

        return Cookies::setRefresh(
            self::session($response, $session),
            $session['refresh_token'],
        );
    }

    /** POST /api/auth/refresh */
    public static function refresh(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        if (self::client($request) === self::LAUNCHER) {
            $session = AuthService::fromConfig()->refresh(
                self::bodyToken($request),
                self::LAUNCHER,
                ClientIp::from($request),
            );

            return Json::respond(
                $response,
                self::sessionBody($session) + ['refresh_token' => $session['refresh_token']],
            );
        }

        $session = AuthService::fromConfig()->refresh(
            Cookies::readRefresh($request),
            self::CLIENT,
            ClientIp::from($request),
        );

        return Cookies::setRefresh(
            self::session($response, $session),
            $session['refresh_token'],
        );
    }

    /** POST /api/auth/logout */
    public static function logout(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        if (self::client($request) === self::LAUNCHER) {
            AuthService::fromConfig()->logout(
                self::bodyToken($request),
                ClientIp::from($request),
                self::LAUNCHER,
            );

            return $response->withStatus(204);
        }

        AuthService::fromConfig()->logout(Cookies::readRefresh($request), ClientIp::from($request));

        return Cookies::clearRefresh($response->withStatus(204));
    }

    /** POST /api/auth/forgot-password */
    public static function forgotPassword(
        ServerRequestInterface $request,
        ResponseInterface $response,
    ): ResponseInterface {
        $body = Body::parse($request);

        AuthService::fromConfig()->requestPasswordReset(self::text($body, 'email'), ClientIp::from($request));

        return Json::respond($response, ['status' => 'check_email'], 202);
    }

    /** POST /api/auth/reset-password */
    public static function resetPassword(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $body = Body::parse($request);

        $token = $body['token'] ?? null;
        if (!is_string($token)) {
            throw new AuthException('invalid_token', 400);
        }

        $password = $body['password'] ?? null;
        if (!is_string($password)) {
            throw new AuthException('missing_field', 400);
        }

        AuthService::fromConfig()->resetPassword($token, $password, ClientIp::from($request));

        return Json::respond($response, ['status' => 'reset']);
    }

    /**
     * A missing or non-string value becomes '', which the service rejects like any other bad input.
     *
     * @param array<string, mixed> $body
     */
    private static function text(array $body, string $key): string
    {
        $value = $body[$key] ?? null;

        return is_string($value) ? $value : '';
    }

    /**
     * The session body. The refresh token and the family id never appear here; the refresh token goes in the cookie.
     *
     * @param array{user: array<string, mixed>, access_token: string, expires_in: int} $session
     */
    private static function session(ResponseInterface $response, array $session): ResponseInterface
    {
        return Json::respond($response, self::sessionBody($session));
    }

    /**
     * @param array{user: array<string, mixed>, access_token: string, expires_in: int} $session
     * @return array{access_token: string, token_type: string, expires_in: int, user: array<string, mixed>}
     */
    private static function sessionBody(array $session): array
    {
        return [
            'access_token' => $session['access_token'],
            'token_type' => 'Bearer',
            'expires_in' => $session['expires_in'],
            'user' => $session['user'],
        ];
    }

    /**
     * The web sends its refresh cookie with X-Lodestone-Client: web. The launcher sends its token in the body.
     *
     * @throws AuthException client_header_missing (403) for any other value
     */
    private static function client(ServerRequestInterface $request): string
    {
        $header = $request->getHeaderLine('X-Lodestone-Client');

        if ($header === self::CLIENT || $header === self::LAUNCHER) {
            return $header;
        }

        throw new AuthException('client_header_missing', 403);
    }

    /** @throws AuthException missing_field (400) when the body has no refresh_token string */
    private static function bodyToken(ServerRequestInterface $request): string
    {
        $body = Body::parse($request);
        $token = $body['refresh_token'] ?? null;

        if (!is_string($token)) {
            throw new AuthException('missing_field', 400);
        }

        return $token;
    }
}
