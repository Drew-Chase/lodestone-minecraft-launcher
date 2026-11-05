<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Billing\Entitlements;
use app\Friends\Gate;
use app\Http\BearerAuth;
use app\Http\Body;
use app\Http\Json;
use app\Storage\BlobStores;
use app\Sync\Access;
use app\Sync\Instances;
use app\Sync\Manifest;
use app\Sync\Quota;
use app\Sync\Uploads;
use app\Sync\Versions;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * Storage, instances, uploads and version history, under /api/storage and /api/instances. Every route needs a bearer
 * access token. Failures are thrown as AuthException, which the error handler turns into {"error": code}.
 */
final class SyncRoute
{
    private const UPLOAD_START_LIMIT = 30;
    private const UPLOAD_WINDOW_SECONDS = 3600;

    public static function configure(App $app): void
    {
        $app->get('/storage', [self::class, 'storage'])->add(new BearerAuth());

        $app->post('/instances', [self::class, 'createInstance'])->add(new BearerAuth());
        $app->get('/instances', [self::class, 'listInstances'])->add(new BearerAuth());
        $app->get('/instances/{id}', [self::class, 'showInstance'])->add(new BearerAuth());
        $app->delete('/instances/{id}', [self::class, 'deleteInstance'])->add(new BearerAuth());

        $app->post('/instances/{id}/uploads', [self::class, 'beginUpload'])->add(new BearerAuth());
        $app->post('/instances/{id}/uploads/{uploadId}/commit', [self::class, 'commitUpload'])->add(new BearerAuth());

        $app->get('/instances/{id}/versions', [self::class, 'listVersions'])->add(new BearerAuth());
        // Registered before /versions/{versionId}, so "latest" is not read as a version id.
        $app->get('/instances/{id}/versions/latest', [self::class, 'latestVersion'])->add(new BearerAuth());
        $app->get('/instances/{id}/versions/{versionId}', [self::class, 'showVersion'])->add(new BearerAuth());
    }

    /** GET /api/storage */
    public static function storage(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::account($request);
        $entitlements = Entitlements::for($userId);

        return Json::respond($response, [
            'used_bytes' => Quota::usedBytes($userId),
            'limit_bytes' => $entitlements['storage_bytes'],
            'plan' => $entitlements['plan'],
            'name' => $entitlements['name'],
        ]);
    }

    /** POST /api/instances */
    public static function createInstance(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::account($request, verified: true);
        self::assertPlan($userId);

        $body = Body::parse($request, optional: true);
        $name = $body['name'] ?? null;
        if (!Instances::validName($name)) {
            throw new AuthException('invalid_name', 400);
        }

        return Json::respond($response, ['instance' => Instances::create($userId, $name)], 201);
    }

    /** GET /api/instances */
    public static function listInstances(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::account($request);

        return Json::respond($response, ['instances' => Instances::listFor($userId)]);
    }

    /** GET /api/instances/{id} */
    public static function showInstance(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request);
        $instanceId = self::id($args);
        $role = self::role($userId, $instanceId);

        $instance = Instances::find($instanceId, $role);
        if ($instance === null) {
            throw new AuthException('not_found', 404);
        }

        return Json::respond($response, ['instance' => $instance]);
    }

    /** DELETE /api/instances/{id} */
    public static function deleteInstance(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request);
        $instanceId = self::id($args);

        if (self::role($userId, $instanceId) !== Access::OWNER) {
            throw new AuthException('forbidden', 403);
        }

        Instances::delete($instanceId);

        return $response->withStatus(204);
    }

    /** POST /api/instances/{id}/uploads */
    public static function beginUpload(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request, verified: true);
        $instanceId = self::id($args);
        self::writer($userId, $instanceId);
        self::assertPlan($userId);

        $store = BlobStores::current();

        // Counts every start that gets past the checks above, including one whose manifest is then refused.
        Gate::limit('sync:upload:' . $userId, self::UPLOAD_START_LIMIT, self::UPLOAD_WINDOW_SECONDS);

        // A body that is not JSON, or is too large to be a manifest, is refused as an invalid manifest.
        try {
            $body = Body::parse($request, maxBytes: Manifest::MAX_BODY_BYTES);
        } catch (AuthException) {
            throw new AuthException('invalid_manifest', 400);
        }

        $files = Manifest::validate($body['files'] ?? null);

        $deviceId = $body['device_id'] ?? null;
        if ($deviceId !== null && (!is_int($deviceId) || $deviceId < 1)) {
            throw new AuthException('invalid_device', 400);
        }

        $started = Uploads::begin($userId, $instanceId, $files, $deviceId, $store);

        return Json::respond($response, $started, 201);
    }

    /** POST /api/instances/{id}/uploads/{uploadId}/commit */
    public static function commitUpload(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request, verified: true);
        $instanceId = self::id($args);
        self::writer($userId, $instanceId);
        self::assertPlan($userId);

        $uploadId = (string) ($args['uploadId'] ?? '');
        if (preg_match('/^[0-9a-f]{32}$/D', $uploadId) !== 1) {
            throw new AuthException('not_found', 404);
        }

        $result = Uploads::commit($userId, $instanceId, $uploadId, BlobStores::current());

        if (isset($result['missing'])) {
            return Json::respond($response, ['error' => 'upload_incomplete', 'missing' => $result['missing']], 409);
        }

        return Json::respond($response, ['version' => $result['version']], 201);
    }

    /** GET /api/instances/{id}/versions */
    public static function listVersions(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request);
        $instanceId = self::id($args);
        self::role($userId, $instanceId);

        return Json::respond($response, ['versions' => Versions::list($instanceId)]);
    }

    /** GET /api/instances/{id}/versions/latest */
    public static function latestVersion(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request);
        $instanceId = self::id($args);
        self::role($userId, $instanceId);

        $latest = Versions::latest($instanceId, BlobStores::current());
        if ($latest === null) {
            throw new AuthException('not_found', 404);
        }

        return Json::respond($response, $latest);
    }

    /** GET /api/instances/{id}/versions/{versionId} */
    public static function showVersion(ServerRequestInterface $request, ResponseInterface $response, array $args): ResponseInterface
    {
        $userId = self::account($request);
        $instanceId = self::id($args);
        self::role($userId, $instanceId);

        $versionId = (string) ($args['versionId'] ?? '');
        if (preg_match('/^[1-9][0-9]{0,18}$/D', $versionId) !== 1) {
            throw new AuthException('not_found', 404);
        }

        $version = Versions::one($instanceId, (int) $versionId, BlobStores::current());
        if ($version === null) {
            throw new AuthException('not_found', 404);
        }

        return Json::respond($response, $version);
    }

    /**
     * The signed-in account. Every route runs it first, so a missing or deleted account is refused before any work.
     *
     * @throws AuthException unauthorized (401), or email_not_verified (403) when $verified is set and the email is not verified
     */
    private static function account(ServerRequestInterface $request, bool $verified = false): int
    {
        $userId = (int) $request->getAttribute('user_id');
        $user = Gate::active($userId);

        if ($verified) {
            Gate::verified($user);
        }

        return $userId;
    }

    /**
     * The user's role on the instance. Any role may read; a user with no role gets 404, so the instance is not revealed.
     *
     * @throws AuthException not_found (404)
     */
    private static function role(int $userId, int $instanceId): string
    {
        $role = Access::roleFor($userId, $instanceId);
        if ($role === null) {
            throw new AuthException('not_found', 404);
        }

        return $role;
    }

    /**
     * The role for a write that owners and editors may make.
     *
     * @throws AuthException not_found (404) or forbidden (403) for a viewer
     */
    private static function writer(int $userId, int $instanceId): string
    {
        $role = self::role($userId, $instanceId);
        if ($role === Access::VIEWER) {
            throw new AuthException('forbidden', 403);
        }

        return $role;
    }

    /** @throws AuthException plan_required (403) when the user's plan has no storage */
    private static function assertPlan(int $userId): void
    {
        if (Entitlements::for($userId)['storage_bytes'] <= 0) {
            throw new AuthException('plan_required', 403);
        }
    }

    /**
     * The instance id from the path. A malformed id is not found, like a missing one.
     *
     * @param array<string, mixed> $args
     * @throws AuthException not_found (404)
     */
    private static function id(array $args): int
    {
        $id = (string) ($args['id'] ?? '');
        if (preg_match('/^[1-9][0-9]{0,18}$/D', $id) !== 1) {
            throw new AuthException('not_found', 404);
        }

        return (int) $id;
    }
}
