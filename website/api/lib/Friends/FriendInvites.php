<?php

declare(strict_types=1);

namespace app\Friends;

use app\Auth\AuthException;
use app\Auth\Tokens;
use app\Db\Database;
use app\Devices\Times;
use app\Users\UserRepository;
use PDO;

/**
 * Single-use invite links. The token is shown once in the link and only its hash is stored.
 */
final class FriendInvites
{
    private const TTL_SECONDS = 604800;

    /** @return array{token: string, expires_at: string} */
    public function create(int $userId): array
    {
        $token = rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
        $pdo = Database::pdo();

        $insert = $pdo->prepare(
            'INSERT INTO friend_invites (inviter_id, token_hash, expires_at)
             VALUES (?, ?, DATE_ADD(NOW(6), INTERVAL ' . self::TTL_SECONDS . ' SECOND))'
        );
        $insert->execute([$userId, Tokens::hash($token)]);

        $select = $pdo->prepare('SELECT expires_at FROM friend_invites WHERE id = ?');
        $select->execute([(int) $pdo->lastInsertId()]);

        return ['token' => $token, 'expires_at' => Times::iso((string) $select->fetchColumn())];
    }

    /**
     * @return array{status: 'accepted', friend: array{username: ?string}}
     * @throws AuthException invalid_invite (404), invite_expired (410), invite_used (409), invalid_friend (400),
     *                       already_friends (409)
     */
    public function accept(int $userId, string $token): array
    {
        $hash = Tokens::hash($token);

        return Database::transaction(function (PDO $pdo) use ($userId, $hash): array {
            $select = $pdo->prepare(
                'SELECT id, inviter_id, used_at, (expires_at <= NOW(6)) AS expired
                 FROM friend_invites WHERE token_hash = ? FOR UPDATE'
            );
            $select->execute([$hash]);
            $invite = $select->fetch(PDO::FETCH_ASSOC);

            if ($invite === false) {
                throw new AuthException('invalid_invite', 404);
            }

            if ((int) $invite['expired'] === 1) {
                throw new AuthException('invite_expired', 410);
            }

            if ($invite['used_at'] !== null) {
                throw new AuthException('invite_used', 409);
            }

            $inviterId = (int) $invite['inviter_id'];
            if ($inviterId === $userId) {
                throw new AuthException('invalid_friend', 400);
            }

            Friendships::lockPair($pdo, $userId, $inviterId);

            $inviter = (new UserRepository())->findById($inviterId);
            if ($inviter === null || $inviter['deleted_at'] !== null) {
                throw new AuthException('invalid_invite', 404);
            }

            (new Friendships())->connectAccepted($pdo, $inviterId, $userId);

            $use = $pdo->prepare('UPDATE friend_invites SET used_by = ?, used_at = NOW(6) WHERE id = ?');
            $use->execute([$userId, (int) $invite['id']]);

            return ['status' => 'accepted', 'friend' => ['username' => Friendships::usernameOf($inviter)]];
        });
    }
}
