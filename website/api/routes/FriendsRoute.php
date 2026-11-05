<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Config\Config;
use app\Devices\ShortCode;
use app\Friends\FriendCodes;
use app\Friends\FriendInvites;
use app\Friends\Friendships;
use app\Friends\Gate;
use app\Http\BearerAuth;
use app\Http\Body;
use app\Http\Json;
use app\Users\UserRepository;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;

/**
 * Friend codes, friend requests, friend lists and invite links, under /api/friends. Every route needs a bearer token.
 */
final class FriendsRoute
{
    private const SEARCH_LIMIT = 30;
    private const REQUEST_LIMIT = 30;
    private const INVITE_LIMIT = 30;
    private const WINDOW_SECONDS = 900;

    public static function configure(App $app): void
    {
        $app->get('/friends/code', [self::class, 'code'])->add(new BearerAuth());
        $app->post('/friends/code/reset', [self::class, 'resetCode'])->add(new BearerAuth());
        $app->get('/friends/search', [self::class, 'search'])->add(new BearerAuth());
        $app->post('/friends/requests', [self::class, 'sendRequest'])->add(new BearerAuth());
        $app->get('/friends', [self::class, 'list'])->add(new BearerAuth());
        $app->post('/friends/requests/{id}/accept', [self::class, 'acceptRequest'])->add(new BearerAuth());
        $app->delete('/friends/{id}', [self::class, 'remove'])->add(new BearerAuth());
        $app->post('/friends/invites', [self::class, 'createInvite'])->add(new BearerAuth());
        $app->post('/friends/invites/accept', [self::class, 'acceptInvite'])->add(new BearerAuth());
    }

    /** GET /api/friends/code */
    public static function code(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        Gate::active(self::userId($request));

        return Json::respond($response, ['code' => (new FriendCodes())->forUser(self::userId($request))]);
    }

    /** POST /api/friends/code/reset */
    public static function resetCode(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        Gate::active(self::userId($request));

        return Json::respond($response, ['code' => (new FriendCodes())->reset(self::userId($request))]);
    }

    /** GET /api/friends/search?username=... */
    public static function search(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::userId($request);
        self::ready($userId);
        Gate::limit('friends:search:' . $userId, self::SEARCH_LIMIT, self::WINDOW_SECONDS);

        $username = $request->getQueryParams()['username'] ?? null;
        if (!is_string($username)) {
            throw new AuthException('missing_field', 400);
        }

        $found = UserRepository::validUsername($username) ? (new UserRepository())->findByUsername($username) : null;
        if ($found === null || $found['deleted_at'] !== null || (int) $found['id'] === $userId) {
            throw new AuthException('not_found', 404);
        }

        return Json::respond($response, ['user' => ['username' => (string) $found['username']]]);
    }

    /** POST /api/friends/requests */
    public static function sendRequest(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::userId($request);
        self::ready($userId);
        Gate::limit('friends:request:' . $userId, self::REQUEST_LIMIT, self::WINDOW_SECONDS);

        $body = Body::parse($request);
        $username = is_string($body['username'] ?? null) ? $body['username'] : null;
        $code = is_string($body['code'] ?? null) ? $body['code'] : null;

        // Exactly one of the two identifies the target.
        if (($username === null) === ($code === null)) {
            throw new AuthException('missing_field', 400);
        }

        $targetId = $username !== null ? self::userIdByUsername($username) : self::userIdByCode($code);
        if ($targetId === null) {
            throw new AuthException('not_found', 404);
        }

        $result = (new Friendships())->request($userId, $targetId);

        if ($result['status'] === 'accepted') {
            return Json::respond($response, ['status' => 'accepted', 'friend' => $result['friend']]);
        }

        return Json::respond($response, [
            'request' => [
                'id' => $result['id'],
                'status' => 'pending',
                'username' => $result['username'],
            ],
        ], 201);
    }

    /** GET /api/friends */
    public static function list(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        Gate::active(self::userId($request));

        return Json::respond($response, (new Friendships())->list(self::userId($request)));
    }

    /** POST /api/friends/requests/{id}/accept */
    public static function acceptRequest(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        $userId = self::userId($request);
        self::ready($userId);

        (new Friendships())->accept($userId, self::friendshipId($args));

        return Json::respond($response, ['status' => 'accepted']);
    }

    /** DELETE /api/friends/{id} */
    public static function remove(
        ServerRequestInterface $request,
        ResponseInterface $response,
        array $args,
    ): ResponseInterface {
        Gate::active(self::userId($request));

        (new Friendships())->remove(self::userId($request), self::friendshipId($args));

        return $response->withStatus(204);
    }

    /** POST /api/friends/invites */
    public static function createInvite(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::userId($request);
        self::ready($userId);
        Gate::limit('friends:invite:' . $userId, self::INVITE_LIMIT, self::WINDOW_SECONDS);

        $invite = (new FriendInvites())->create($userId);
        $base = rtrim(Config::need('APP_URL'), '/');

        return Json::respond($response, [
            'url' => $base . '/friends/invite#token=' . $invite['token'],
            'expires_at' => $invite['expires_at'],
        ], 201);
    }

    /** POST /api/friends/invites/accept */
    public static function acceptInvite(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $userId = self::userId($request);
        self::ready($userId);

        $body = Body::parse($request);
        $token = $body['token'] ?? null;
        if (!is_string($token)) {
            throw new AuthException('missing_field', 400);
        }

        return Json::respond($response, (new FriendInvites())->accept($userId, $token));
    }

    private static function userId(ServerRequestInterface $request): int
    {
        return (int) $request->getAttribute('user_id');
    }

    /**
     * The checks for routes that need a verified email and a username.
     *
     * @throws AuthException unauthorized (401), email_not_verified (403) or username_required (409)
     */
    private static function ready(int $userId): void
    {
        $user = Gate::active($userId);
        Gate::verified($user);
        Gate::named($user);
    }

    /** The id of the account with this username, or null when none has it. */
    private static function userIdByUsername(string $username): ?int
    {
        if (!UserRepository::validUsername($username)) {
            return null;
        }

        $user = (new UserRepository())->findByUsername($username);

        return $user === null ? null : (int) $user['id'];
    }

    private static function userIdByCode(string $code): ?int
    {
        $normalized = ShortCode::normalize($code);

        return $normalized === null ? null : (new FriendCodes())->ownerId($normalized);
    }

    /** @param array<string, mixed> $args @throws AuthException not_found (404) for a malformed id */
    private static function friendshipId(array $args): int
    {
        $id = (string) ($args['id'] ?? '');
        if (preg_match('/^[1-9][0-9]{0,18}$/D', $id) !== 1) {
            throw new AuthException('not_found', 404);
        }

        return (int) $id;
    }
}
