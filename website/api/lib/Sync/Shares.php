<?php

declare(strict_types=1);

namespace app\Sync;

use app\Auth\AuthException;
use app\Billing\Entitlements;
use app\Db\Database;
use app\Devices\Times;
use app\Friends\Gate;
use app\Users\UserRepository;
use PDO;

/**
 * Sharing an instance with a friend as a viewer or an editor. Only the owner grants, changes or removes a share.
 * A new share on an instance with no shares yet counts toward the owner's plan cap.
 */
final class Shares
{
    private const ACCESS_LEVELS = [Access::VIEWER, Access::EDITOR];

    /**
     * Grants a share, or changes the access of a share the friend already has. The cap is checked under the owner's
     * row lock, so two grants at once cannot both take the last slot.
     *
     * @return array{created: bool, share: array{username: string, access: string, granted_at: string}}
     * @throws AuthException not_found (404), forbidden (403), email_not_verified (403), invalid_access (400),
     *                       missing_field (400), invalid_share (400), sharing_not_allowed (403),
     *                       share_limit_reached (409)
     */
    public function grant(int $actorId, int $instanceId, mixed $username, mixed $access): array
    {
        $owner = $this->ownerOf($actorId, $instanceId);
        Gate::verified($owner);

        $level = self::accessLevel($access);
        $friend = $this->friendNamed($username);
        $friendId = (int) $friend['id'];

        if ($friendId === $actorId) {
            throw new AuthException('invalid_share', 400);
        }

        return Database::transaction(function (PDO $pdo) use ($actorId, $instanceId, $friend, $level): array {
            $pdo->prepare('SELECT id FROM users WHERE id = ? FOR UPDATE')->execute([$actorId]);

            // Checked under a lock on the friendship row, so a removal of the friendship waits for this share to commit
            // and then drops it, instead of landing between the check and the insert.
            if (!self::areFriends($pdo, $actorId, (int) $friend['id'])) {
                throw new AuthException('not_found', 404);
            }

            $cap = Entitlements::for($actorId)['shares'];
            if ($cap === 0) {
                throw new AuthException('sharing_not_allowed', 403);
            }

            $friendId = (int) $friend['id'];
            $existing = $pdo->prepare(
                'SELECT granted_at FROM instance_shares WHERE instance_id = ? AND friend_user_id = ? FOR UPDATE'
            );
            $existing->execute([$instanceId, $friendId]);
            $grantedAt = $existing->fetchColumn();

            if ($grantedAt !== false) {
                $update = $pdo->prepare('UPDATE instance_shares SET access = ? WHERE instance_id = ? AND friend_user_id = ?');
                $update->execute([$level, $instanceId, $friendId]);

                return ['created' => false, 'share' => self::shareBody((string) $friend['username'], $level, (string) $grantedAt)];
            }

            if ($cap !== null && !self::hasShares($pdo, $instanceId) && self::sharedInstances($pdo, $actorId) + 1 > $cap) {
                throw new AuthException('share_limit_reached', 409);
            }

            $insert = $pdo->prepare('INSERT INTO instance_shares (instance_id, friend_user_id, access) VALUES (?, ?, ?)');
            $insert->execute([$instanceId, $friendId, $level]);

            $select = $pdo->prepare('SELECT granted_at FROM instance_shares WHERE instance_id = ? AND friend_user_id = ?');
            $select->execute([$instanceId, $friendId]);

            return ['created' => true, 'share' => self::shareBody((string) $friend['username'], $level, (string) $select->fetchColumn())];
        });
    }

    /**
     * The shares of an instance, oldest first.
     *
     * @return list<array{username: string, access: string, granted_at: string}>
     * @throws AuthException not_found (404) or forbidden (403)
     */
    public function list(int $actorId, int $instanceId): array
    {
        $this->ownerOf($actorId, $instanceId);

        $statement = Database::pdo()->prepare(
            'SELECT u.username, s.access, s.granted_at FROM instance_shares s
             JOIN users u ON u.id = s.friend_user_id
             WHERE s.instance_id = ?
             ORDER BY s.granted_at, s.friend_user_id'
        );
        $statement->execute([$instanceId]);

        $shares = [];
        foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
            $shares[] = self::shareBody((string) $row['username'], (string) $row['access'], (string) $row['granted_at']);
        }

        return $shares;
    }

    /**
     * Removes a share. The username is matched the way accounts are looked up, so case does not matter.
     *
     * @throws AuthException not_found (404) when no share goes to that username, forbidden (403) for a non-owner
     */
    public function revoke(int $actorId, int $instanceId, string $username): void
    {
        $this->ownerOf($actorId, $instanceId);

        $statement = Database::pdo()->prepare(
            'DELETE s FROM instance_shares s
             JOIN users u ON u.id = s.friend_user_id
             WHERE s.instance_id = ? AND u.username = ?'
        );
        $statement->execute([$instanceId, $username]);

        if ($statement->rowCount() !== 1) {
            throw new AuthException('not_found', 404);
        }
    }

    /**
     * Drops every share between two accounts, in either direction: each one owning an instance shared with the other.
     * The caller holds the transaction in which the friendship changes.
     */
    public static function dropBetween(PDO $pdo, int $first, int $second): void
    {
        $statement = $pdo->prepare(
            'DELETE s FROM instance_shares s
             JOIN instances i ON i.id = s.instance_id
             WHERE (i.owner_id = ? AND s.friend_user_id = ?) OR (i.owner_id = ? AND s.friend_user_id = ?)'
        );
        $statement->execute([$first, $second, $second, $first]);
    }

    /**
     * The signed-in account, after checking it is the instance's owner.
     *
     * @return array<string, mixed>
     * @throws AuthException unauthorized (401), not_found (404) for no role, forbidden (403) for an editor or viewer
     */
    private function ownerOf(int $actorId, int $instanceId): array
    {
        $user = Gate::active($actorId);
        $role = Access::roleFor($actorId, $instanceId);

        if ($role === null) {
            throw new AuthException('not_found', 404);
        }

        if ($role !== Access::OWNER) {
            throw new AuthException('forbidden', 403);
        }

        return $user;
    }

    /** @return 'viewer'|'editor' @throws AuthException invalid_access (400) */
    private static function accessLevel(mixed $access): string
    {
        if (is_string($access) && in_array($access, self::ACCESS_LEVELS, true)) {
            return $access;
        }

        throw new AuthException('invalid_access', 400);
    }

    /**
     * @return array<string, mixed>
     * @throws AuthException missing_field (400) when no username is given, not_found (404) when no account has it
     */
    private function friendNamed(mixed $username): array
    {
        if (!is_string($username) || trim($username) === '') {
            throw new AuthException('missing_field', 400);
        }

        $friend = (new UserRepository())->findByUsername($username);
        if ($friend === null || $friend['deleted_at'] !== null) {
            throw new AuthException('not_found', 404);
        }

        return $friend;
    }

    /**
     * True when an accepted friendship joins the pair, whichever way it was started. The row is locked until the caller's
     * transaction ends.
     */
    private static function areFriends(PDO $pdo, int $first, int $second): bool
    {
        $statement = $pdo->prepare(
            "SELECT 1 FROM friendships
             WHERE status = 'accepted'
               AND ((requester_id = ? AND addressee_id = ?) OR (requester_id = ? AND addressee_id = ?))
             LIMIT 1 FOR UPDATE"
        );
        $statement->execute([$first, $second, $second, $first]);

        return $statement->fetchColumn() !== false;
    }

    private static function hasShares(PDO $pdo, int $instanceId): bool
    {
        $statement = $pdo->prepare('SELECT COUNT(*) FROM instance_shares WHERE instance_id = ?');
        $statement->execute([$instanceId]);

        return (int) $statement->fetchColumn() > 0;
    }

    /** The number of distinct instances the owner has at least one share on. */
    private static function sharedInstances(PDO $pdo, int $ownerId): int
    {
        $statement = $pdo->prepare(
            'SELECT COUNT(DISTINCT s.instance_id) FROM instance_shares s
             JOIN instances i ON i.id = s.instance_id
             WHERE i.owner_id = ?'
        );
        $statement->execute([$ownerId]);

        return (int) $statement->fetchColumn();
    }

    /** @return array{username: string, access: string, granted_at: string} */
    private static function shareBody(string $username, string $access, string $grantedAt): array
    {
        return [
            'username' => $username,
            'access' => $access,
            'granted_at' => Times::iso($grantedAt) ?? $grantedAt,
        ];
    }
}
