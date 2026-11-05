<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Auth\OAuth\Transports;
use app\Auth\RateLimiter;
use app\Friends\Gate;
use app\Http\BearerAuth;
use app\Http\Json;
use app\Minecraft\LinkService;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * Minecraft account linking, under /api/minecraft. Every route needs a bearer access token. Failures are thrown
 * as AuthException, which the error handler turns into the JSON error.
 */
final class MinecraftRoute
{
    private const LINK_LIMIT = 10;
    private const LINK_WINDOW_SECONDS = 3600;

    public static function configure(App $app): void
    {
        $app->post('/minecraft/link', [self::class, 'startLink'])->add(new BearerAuth());
        $app->get('/minecraft/link/{id}', [self::class, 'linkStatus'])->add(new BearerAuth());
        $app->get('/minecraft/accounts', [self::class, 'accounts'])->add(new BearerAuth());
        $app->delete('/minecraft/accounts/{uuid}', [self::class, 'unlink'])->add(new BearerAuth());
    }

    /** POST /api/minecraft/link */
    public static function startLink(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::userId($request);

        // Every start counts, including the ones that fail later, so a user cannot spin up sign-ins without limit.
        $limiter = new RateLimiter();
        if ($limiter->hit('minecraft:link:' . $userId, self::LINK_WINDOW_SECONDS) > self::LINK_LIMIT) {
            throw new AuthException('rate_limited', 429, $limiter->retryAfter(self::LINK_WINDOW_SECONDS));
        }

        $started = self::service()->start($userId);

        return Json::respond($response, $started, 201);
    }

    /** GET /api/minecraft/link/{id} */
    public static function linkStatus(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        $status = self::service()->status(self::userId($request), (string) $args['id']);

        return Json::respond($response, $status);
    }

    /** GET /api/minecraft/accounts */
    public static function accounts(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        return Json::respond($response, self::service()->accounts(self::userId($request)));
    }

    /** DELETE /api/minecraft/accounts/{uuid} */
    public static function unlink(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        self::service()->unlink(self::userId($request), (string) $args['uuid']);

        return $response->withStatus(204);
    }

    private static function service(): LinkService
    {
        return new LinkService(Transports::current());
    }

    /**
     * The id of the signed-in account. Every action takes it here, so a missing or deleted account is refused
     * before any work is done.
     *
     * @throws AuthException unauthorized (401) when the account is missing or deleted
     */
    private static function userId(ServerRequestInterface $request): int
    {
        return (int) Gate::active((int) $request->getAttribute('user_id'))['id'];
    }
}
