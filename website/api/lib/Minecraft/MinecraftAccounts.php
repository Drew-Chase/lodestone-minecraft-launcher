<?php

declare(strict_types=1);

namespace app\Minecraft;

use app\Db\Database;
use PDO;
use PDOException;

/**
 * Reads and writes minecraft_accounts. A uuid belongs to one user at a time.
 */
final class MinecraftAccounts
{
    private const DUPLICATE_KEY = 1062;
    private const DEADLOCK = 1213;
    private const WRITE_ATTEMPTS = 3;

    /** @return list<array{uuid: string, name: string, linked_at: string}> */
    public function forUser(int $userId): array
    {
        $statement = Database::pdo()->prepare(
            'SELECT uuid, name, linked_at FROM minecraft_accounts WHERE user_id = ? ORDER BY linked_at, uuid'
        );
        $statement->execute([$userId]);

        return array_map(
            static fn (array $row): array => self::shape($row),
            $statement->fetchAll(PDO::FETCH_ASSOC),
        );
    }

    /**
     * Links the account to the user, or refreshes its name and link time when the user already holds it.
     * Returns null, and writes nothing, when another user holds the uuid.
     *
     * The row is inserted before anything locks the uuid, so no lock is taken on a uuid that does not exist yet.
     * A duplicate key names the owner, read with a plain SELECT. A deadlock reruns the whole step once.
     *
     * @return array{uuid: string, name: string, linked_at: string}|null
     */
    public function link(int $userId, string $uuid, string $name): ?array
    {
        try {
            return $this->linkInTransaction($userId, $uuid, $name);
        } catch (PDOException $error) {
            if (self::driverCode($error) !== self::DEADLOCK) {
                throw $error;
            }
        }

        return $this->linkInTransaction($userId, $uuid, $name);
    }

    /** True when the user held the account and it is now removed. */
    public function unlink(int $userId, string $uuid): bool
    {
        $statement = Database::pdo()->prepare('DELETE FROM minecraft_accounts WHERE uuid = ? AND user_id = ?');
        $statement->execute([$uuid, $userId]);

        return $statement->rowCount() === 1;
    }

    /** @return array{uuid: string, name: string, linked_at: string}|null */
    private function linkInTransaction(int $userId, string $uuid, string $name): ?array
    {
        return Database::transaction(fn (PDO $pdo): ?array => $this->write($pdo, $userId, $uuid, $name));
    }

    /** @return array{uuid: string, name: string, linked_at: string}|null */
    private function write(PDO $pdo, int $userId, string $uuid, string $name): ?array
    {
        $insert = $pdo->prepare(
            'INSERT INTO minecraft_accounts (uuid, user_id, name, linked_at) VALUES (?, ?, ?, NOW(6))'
        );
        $refresh = $pdo->prepare('UPDATE minecraft_accounts SET name = ?, linked_at = NOW(6) WHERE uuid = ?');

        // A row removed between the duplicate and the read leaves nothing to refresh, so the insert is tried again.
        for ($attempt = 0; $attempt < self::WRITE_ATTEMPTS; $attempt++) {
            try {
                $insert->execute([$uuid, $userId, $name]);
            } catch (PDOException $error) {
                if (self::driverCode($error) !== self::DUPLICATE_KEY) {
                    throw $error;
                }

                $holder = $this->holder($pdo, $uuid);
                if ($holder === null) {
                    continue;
                }
                if ($holder !== $userId) {
                    return null;
                }

                $refresh->execute([$name, $uuid]);
            }

            return $this->read($pdo, $uuid);
        }

        throw new \RuntimeException('The Minecraft account changed too often to be linked');
    }

    /** The user who holds the uuid, or null when no row has it. A plain read, so it takes no lock. */
    private function holder(PDO $pdo, string $uuid): ?int
    {
        $statement = $pdo->prepare('SELECT user_id FROM minecraft_accounts WHERE uuid = ?');
        $statement->execute([$uuid]);
        $holder = $statement->fetchColumn();

        return $holder === false ? null : (int) $holder;
    }

    /** @return array{uuid: string, name: string, linked_at: string} */
    private function read(PDO $pdo, string $uuid): array
    {
        $statement = $pdo->prepare('SELECT uuid, name, linked_at FROM minecraft_accounts WHERE uuid = ?');
        $statement->execute([$uuid]);

        return self::shape($statement->fetch(PDO::FETCH_ASSOC));
    }

    /** The MySQL error number of a PDO failure, or 0 when the driver gave none. */
    private static function driverCode(PDOException $error): int
    {
        return (int) ($error->errorInfo[1] ?? 0);
    }

    /**
     * @param array<string, mixed> $row
     * @return array{uuid: string, name: string, linked_at: string}
     */
    private static function shape(array $row): array
    {
        return [
            'uuid' => (string) $row['uuid'],
            'name' => (string) $row['name'],
            'linked_at' => Times::iso((string) $row['linked_at']),
        ];
    }
}
