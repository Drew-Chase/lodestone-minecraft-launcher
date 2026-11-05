<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Config\Config;
use app\Devices\DeviceFields;
use app\Devices\DeviceLinks;
use app\Devices\DeviceTokens;
use app\Devices\Devices;
use app\Friends\Gate;
use app\Http\BearerAuth;
use app\Http\Body;
use app\Http\ClientIp;
use app\Http\Json;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * Device sign-in for the launcher, and the list of its devices. Under /api/device and /api/devices.
 */
final class DeviceRoute
{
    private const START_LIMIT = 10;
    private const START_WINDOW_SECONDS = 900;
    private const APPROVE_LIMIT = 30;
    private const APPROVE_WINDOW_SECONDS = 900;
    private const LOOKUP_LIMIT = 30;
    private const LOOKUP_WINDOW_SECONDS = 900;

    public static function configure(App $app): void
    {
        $app->post('/device/start', [self::class, 'start']);
        $app->post('/device/lookup', [self::class, 'lookup'])->add(new BearerAuth());
        $app->post('/device/approve', [self::class, 'approve'])->add(new BearerAuth());
        $app->post('/device/token', [self::class, 'token']);
        $app->get('/devices', [self::class, 'devices'])->add(new BearerAuth());
        $app->delete('/devices/{id}', [self::class, 'revoke'])->add(new BearerAuth());
    }

    /** POST /api/device/start */
    public static function start(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        Gate::limit('device:start:' . ClientIp::from($request), self::START_LIMIT, self::START_WINDOW_SECONDS);

        $fields = DeviceFields::read(Body::parse($request));
        $started = (new DeviceLinks())->start($fields);

        $base = rtrim(Config::need('APP_URL'), '/');

        return Json::respond($response, [
            'device_code' => $started['device_code'],
            'user_code' => $started['user_code'],
            'verification_uri' => $base . '/link',
            'verification_uri_complete' => $base . '/link?code=' . $started['user_code'],
            'expires_in' => DeviceLinks::TTL_SECONDS,
            'interval' => DeviceLinks::POLL_INTERVAL_SECONDS,
        ]);
    }

    /** POST /api/device/lookup */
    public static function lookup(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $user = Gate::active((int) $request->getAttribute('user_id'));
        Gate::limit('device:lookup:' . (int) $user['id'], self::LOOKUP_LIMIT, self::LOOKUP_WINDOW_SECONDS);

        $body = Body::parse($request);
        $userCode = $body['user_code'] ?? null;
        if (!is_string($userCode)) {
            throw new AuthException('missing_field', 400);
        }

        return Json::respond($response, (new DeviceLinks())->lookup($userCode));
    }

    /** POST /api/device/approve */
    public static function approve(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $user = Gate::active((int) $request->getAttribute('user_id'));
        Gate::limit('device:approve:' . (int) $user['id'], self::APPROVE_LIMIT, self::APPROVE_WINDOW_SECONDS);

        $body = Body::parse($request);
        $userCode = $body['user_code'] ?? null;
        if (!is_string($userCode)) {
            throw new AuthException('missing_field', 400);
        }

        (new DeviceLinks())->approve((int) $user['id'], $userCode);

        return Json::respond($response, ['status' => 'approved']);
    }

    /** POST /api/device/token */
    public static function token(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $body = Body::parse($request);
        $deviceCode = $body['device_code'] ?? null;
        if (!is_string($deviceCode)) {
            throw new AuthException('missing_field', 400);
        }

        $session = (new DeviceTokens())->redeem($deviceCode, ClientIp::from($request));

        return Json::respond($response, [
            'access_token' => $session['access_token'],
            'token_type' => 'Bearer',
            'expires_in' => $session['expires_in'],
            'refresh_token' => $session['refresh_token'],
            'user' => $session['user'],
            'device' => $session['device'],
        ]);
    }

    /** GET /api/devices */
    public static function devices(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = (int) $request->getAttribute('user_id');
        Gate::active($userId);

        $devices = new Devices();

        return Json::respond($response, [
            'devices' => $devices->active($userId),
            'limit' => $devices->limit($userId),
        ]);
    }

    /** DELETE /api/devices/{id} */
    public static function revoke(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        $userId = (int) $request->getAttribute('user_id');
        Gate::active($userId);

        $id = (string) ($args['id'] ?? '');
        if (preg_match('/^[1-9][0-9]{0,18}$/D', $id) !== 1) {
            throw new AuthException('not_found', 404);
        }

        (new Devices())->revoke($userId, (int) $id);

        return $response->withStatus(204);
    }
}
