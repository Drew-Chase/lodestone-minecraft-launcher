<?php

declare(strict_types=1);

namespace app\Friends;

use app\Db\Database;
use app\Devices\ShortCode;
use PDO;
use PDOException;
use RuntimeException;

/**
 * Each account has one friend code. Another person enters it to send a friend request without knowing the username.
 */
final class FriendCodes
{
    private const ATTEMPTS = 5;

    /** The account's code, created on first use. */
    public function forUser(int $userId): string
    {
        return $this->find($userId) ?? $this->create($userId);
    }

    /** Replaces the code. The old one stops working at once. */
    public function reset(int $userId): string
    {
        return Database::transaction(function (PDO $pdo) use ($userId): string {
            $delete = $pdo->prepare('DELETE FROM friend_codes WHERE user_id = ?');
            $delete->execute([$userId]);

            return $this->create($userId);
        });
    }

    /** The account that owns the code, or null. The code is already normalised. */
    public function ownerId(string $code): ?int
    {
        $statement = Database::pdo()->prepare('SELECT user_id FROM friend_codes WHERE code = ?');
        $statement->execute([$code]);
        $owner = $statement->fetchColumn();

        return $owner === false ? null : (int) $owner;
    }

    private function find(int $userId): ?string
    {
        $statement = Database::pdo()->prepare('SELECT code FROM friend_codes WHERE user_id = ?');
        $statement->execute([$userId]);
        $code = $statement->fetchColumn();

        return $code === false ? null : (string) $code;
    }

    private function create(int $userId): string
    {
        $pdo = Database::pdo();

        for ($attempt = 0; $attempt < self::ATTEMPTS; $attempt++) {
            $code = ShortCode::random();

            try {
                $insert = $pdo->prepare('INSERT INTO friend_codes (user_id, code) VALUES (?, ?)');
                $insert->execute([$userId, $code]);

                return $code;
            } catch (PDOException $error) {
                if ($error->getCode() !== '23000') {
                    throw $error;
                }

                // A concurrent request may have created the account's code. Otherwise the code was taken, so draw again.
                $existing = $this->find($userId);
                if ($existing !== null) {
                    return $existing;
                }
            }
        }

        throw new RuntimeException('Could not allocate a friend code');
    }
}
