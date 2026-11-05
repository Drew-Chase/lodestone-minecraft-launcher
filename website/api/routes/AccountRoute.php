<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Auth\AuthService;
use app\Friends\Gate;
use app\Http\BearerAuth;
use app\Http\Body;
use app\Http\ClientIp;
use app\Http\Json;
use app\Users\UserRepository;
use PDOException;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * Routes that need a bearer access token. The middleware is attached to each route, not to the app.
 */
final class AccountRoute
{
    public static function configure(App $app): void
    {
        $app->get('/me', [self::class, 'me'])->add(new BearerAuth());
        $app->post('/auth/change-password', [self::class, 'changePassword'])->add(new BearerAuth());
        $app->get('/account/sessions', [self::class, 'sessions'])->add(new BearerAuth());
        $app->delete('/account/sessions/{id}', [self::class, 'revokeSession'])->add(new BearerAuth());
        $app->post('/account/sessions/revoke-others', [self::class, 'revokeOtherSessions'])->add(new BearerAuth());
        $app->post('/account/username', [self::class, 'username'])->add(new BearerAuth());
    }

    /** POST /api/account/username */
    public static function username(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = (int) $request->getAttribute('user_id');
        Gate::active($userId);

        $body = Body::parse($request);
        $username = $body['username'] ?? null;
        if (!is_string($username)) {
            throw new AuthException('missing_field', 400);
        }

        if (!UserRepository::validUsername($username)) {
            throw new AuthException('invalid_username', 400);
        }

        $users = new UserRepository();
        $holder = $users->findByUsername($username);
        if ($holder !== null && (int) $holder['id'] !== $userId) {
            throw new AuthException('username_taken', 409);
        }

        try {
            $users->setUsername($userId, $username);
        } catch (PDOException $error) {
            // Another account took the name between the lookup and the update.
            if ($error->getCode() === '23000') {
                throw new AuthException('username_taken', 409);
            }

            throw $error;
        }

        return Json::respond($response, ['username' => $username]);
    }

    /** GET /api/me */
    public static function me(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $user = AuthService::fromConfig()->currentUser((int) $request->getAttribute('user_id'));

        return Json::respond($response, ['user' => $user]);
    }

    /** POST /api/auth/change-password */
    public static function changePassword(
        ServerRequestInterface $request,
        ResponseInterface $response,
    ): ResponseInterface {
        $body = Body::parse($request);

        $current = $body['current_password'] ?? null;
        if (!is_string($current)) {
            throw new AuthException('missing_field', 400);
        }

        $new = $body['new_password'] ?? null;
        if (!is_string($new)) {
            throw new AuthException('missing_field', 400);
        }

        AuthService::fromConfig()->changePassword(
            (int) $request->getAttribute('user_id'),
            $current,
            $new,
            (string) $request->getAttribute('family_id'),
            ClientIp::from($request),
        );

        return Json::respond($response, ['status' => 'changed']);
    }

    /** GET /api/account/sessions */
    public static function sessions(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $sessions = AuthService::fromConfig()->listSessions(
            (int) $request->getAttribute('user_id'),
            (string) $request->getAttribute('family_id'),
        );

        return Json::respond($response, $sessions);
    }

    /** DELETE /api/account/sessions/{id} */
    public static function revokeSession(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        AuthService::fromConfig()->revokeSession(
            (int) $request->getAttribute('user_id'),
            (string) $args['id'],
            (string) $request->getAttribute('family_id'),
            ClientIp::from($request),
        );

        return $response->withStatus(204);
    }

    /** POST /api/account/sessions/revoke-others */
    public static function revokeOtherSessions(
        ServerRequestInterface $request,
        ResponseInterface $response,
    ): ResponseInterface {
        $count = AuthService::fromConfig()->revokeOtherSessions(
            (int) $request->getAttribute('user_id'),
            (string) $request->getAttribute('family_id'),
            ClientIp::from($request),
        );

        return Json::respond($response, ['status' => 'revoked', 'count' => $count]);
    }
}
