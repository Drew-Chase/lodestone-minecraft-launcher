<?php

declare(strict_types=1);

namespace app\Friends;

use app\Auth\AuthException;
use app\Db\Database;
use app\Users\UserRepository;
use PDO;

/**
 * Friend requests and friendships. A pair of accounts has at most one row, whichever way it was started.
 */
final class Friendships
{
    /**
     * @return array{friends: list<array{id: int, username: ?string}>, incoming: list<array{id: int, username: ?string}>, outgoing: list<array{id: int, username: ?string}>}
     */
    public function list(int $userId): array
    {
        return [
            'friends' => $this->rows(
                "SELECT f.id, u.username FROM friendships f
                 JOIN users u ON u.id = IF(f.requester_id = ?, f.addressee_id, f.requester_id)
                 WHERE (f.requester_id = ? OR f.addressee_id = ?) AND f.status = 'accepted' AND u.deleted_at IS NULL
                 ORDER BY f.id",
                [$userId, $userId, $userId],
            ),
            'incoming' => $this->rows(
                "SELECT f.id, u.username FROM friendships f
                 JOIN users u ON u.id = f.requester_id
                 WHERE f.addressee_id = ? AND f.status = 'pending' AND u.deleted_at IS NULL
                 ORDER BY f.id",
                [$userId],
            ),
            'outgoing' => $this->rows(
                "SELECT f.id, u.username FROM friendships f
                 JOIN users u ON u.id = f.addressee_id
                 WHERE f.requester_id = ? AND f.status = 'pending' AND u.deleted_at IS NULL
                 ORDER BY f.id",
                [$userId],
            ),
        ];
    }

    /**
     * Sends a request. When the target has already asked this user, the request is accepted instead.
     *
     * @return array{status: 'accepted', friend: array{username: ?string}}|array{status: 'pending', id: int, username: ?string}
     * @throws AuthException not_found (404), invalid_friend (400), already_friends (409) or request_pending (409)
     */
    public function request(int $userId, int $targetId): array
    {
        $target = (new UserRepository())->findById($targetId);
        if ($target === null || $target['deleted_at'] !== null) {
            throw new AuthException('not_found', 404);
        }

        if ($targetId === $userId) {
            throw new AuthException('invalid_friend', 400);
        }

        $name = self::usernameOf($target);

        return Database::transaction(function (PDO $pdo) use ($userId, $targetId, $name): array {
            self::lockPair($pdo, $userId, $targetId);
            $existing = self::between($pdo, $userId, $targetId);

            if ($existing !== null) {
                if ($existing['status'] === 'accepted') {
                    throw new AuthException('already_friends', 409);
                }

                if ((int) $existing['requester_id'] === $userId) {
                    throw new AuthException('request_pending', 409);
                }

                $accept = $pdo->prepare(
                    "UPDATE friendships SET status = 'accepted', accepted_at = NOW(6) WHERE id = ?"
                );
                $accept->execute([(int) $existing['id']]);

                return ['status' => 'accepted', 'friend' => ['username' => $name]];
            }

            $insert = $pdo->prepare("INSERT INTO friendships (requester_id, addressee_id, status) VALUES (?, ?, 'pending')");
            $insert->execute([$userId, $targetId]);

            return ['status' => 'pending', 'id' => (int) $pdo->lastInsertId(), 'username' => $name];
        });
    }

    /**
     * Accepts a request that was sent to the user. Only the addressee may accept.
     *
     * @throws AuthException not_found (404) when no pending request of the user has this id
     */
    public function accept(int $userId, int $friendshipId): void
    {
        $statement = Database::pdo()->prepare(
            "UPDATE friendships SET status = 'accepted', accepted_at = NOW(6)
             WHERE id = ? AND addressee_id = ? AND status = 'pending'"
        );
        $statement->execute([$friendshipId, $userId]);

        if ($statement->rowCount() !== 1) {
            throw new AuthException('not_found', 404);
        }
    }

    /**
     * Ends a friendship, cancels a request or declines one. Either party may do it.
     *
     * @throws AuthException not_found (404) when the user is not a party to the row
     */
    public function remove(int $userId, int $friendshipId): void
    {
        $statement = Database::pdo()->prepare(
            'DELETE FROM friendships WHERE id = ? AND (requester_id = ? OR addressee_id = ?)'
        );
        $statement->execute([$friendshipId, $userId, $userId]);

        if ($statement->rowCount() !== 1) {
            throw new AuthException('not_found', 404);
        }
    }

    /**
     * Makes the pair friends, as an invite does. The caller holds the pair's locks and the transaction.
     *
     * @throws AuthException already_friends (409)
     */
    public function connectAccepted(PDO $pdo, int $requesterId, int $addresseeId): void
    {
        $existing = self::between($pdo, $requesterId, $addresseeId);

        if ($existing !== null) {
            if ($existing['status'] === 'accepted') {
                throw new AuthException('already_friends', 409);
            }

            $accept = $pdo->prepare("UPDATE friendships SET status = 'accepted', accepted_at = NOW(6) WHERE id = ?");
            $accept->execute([(int) $existing['id']]);

            return;
        }

        $insert = $pdo->prepare(
            "INSERT INTO friendships (requester_id, addressee_id, status, accepted_at) VALUES (?, ?, 'accepted', NOW(6))"
        );
        $insert->execute([$requesterId, $addresseeId]);
    }

    /**
     * Locks both account rows in id order, so two requests between the same pair run one after the other.
     */
    public static function lockPair(PDO $pdo, int $first, int $second): void
    {
        $statement = $pdo->prepare('SELECT id FROM users WHERE id IN (?, ?) ORDER BY id FOR UPDATE');
        $statement->execute([min($first, $second), max($first, $second)]);
    }

    /** @param array<string, mixed> $user */
    public static function usernameOf(array $user): ?string
    {
        $username = $user['username'] ?? null;

        return $username === null ? null : (string) $username;
    }

    /** @return array{id: int, requester_id: int, status: string}|null */
    private static function between(PDO $pdo, int $first, int $second): ?array
    {
        $statement = $pdo->prepare(
            'SELECT id, requester_id, status FROM friendships
             WHERE (requester_id = ? AND addressee_id = ?) OR (requester_id = ? AND addressee_id = ?) LIMIT 1'
        );
        $statement->execute([$first, $second, $second, $first]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        if ($row === false) {
            return null;
        }

        return [
            'id' => (int) $row['id'],
            'requester_id' => (int) $row['requester_id'],
            'status' => (string) $row['status'],
        ];
    }

    /**
     * @param list<int|string> $params
     * @return list<array{id: int, username: ?string}>
     */
    private function rows(string $sql, array $params): array
    {
        $statement = Database::pdo()->prepare($sql);
        $statement->execute($params);

        $rows = [];
        foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
            $rows[] = [
                'id' => (int) $row['id'],
                'username' => $row['username'] === null ? null : (string) $row['username'],
            ];
        }

        return $rows;
    }
}
