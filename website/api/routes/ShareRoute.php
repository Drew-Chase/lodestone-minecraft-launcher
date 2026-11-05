<?php

declare(strict_types=1);

namespace routes;

use app\Http\BearerAuth;
use app\Http\Body;
use app\Http\Json;
use app\Sync\Shares;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * Sharing routes under /api/instances/{id}/shares. Every route needs a bearer token. Only the owner may use them;
 * the rules and the errors live in app\Sync\Shares.
 */
final class ShareRoute
{
    public static function configure(App $app): void
    {
        $app->post('/instances/{id}/shares', [self::class, 'grant'])->add(new BearerAuth());
        $app->get('/instances/{id}/shares', [self::class, 'list'])->add(new BearerAuth());
        $app->delete('/instances/{id}/shares/{username}', [self::class, 'revoke'])->add(new BearerAuth());
    }

    /** POST /api/instances/{id}/shares */
    public static function grant(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        $body = Body::parse($request);

        $result = (new Shares())->grant(
            self::userId($request),
            (int) $args['id'],
            $body['username'] ?? null,
            $body['access'] ?? null,
        );

        return Json::respond($response, ['share' => $result['share']], $result['created'] ? 201 : 200);
    }

    /** GET /api/instances/{id}/shares */
    public static function list(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        return Json::respond($response, ['shares' => (new Shares())->list(self::userId($request), (int) $args['id'])]);
    }

    /** DELETE /api/instances/{id}/shares/{username} */
    public static function revoke(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        (new Shares())->revoke(self::userId($request), (int) $args['id'], (string) $args['username']);

        return $response->withStatus(204);
    }

    private static function userId(ServerRequestInterface $request): int
    {
        return (int) $request->getAttribute('user_id');
    }
}
