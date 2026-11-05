<?php

declare(strict_types=1);

namespace app\Sync;

use app\Auth\AuthException;
use app\Db\Database;
use PDO;

/** Instances the user owns or has been shared, and their latest version. Access checks go through Access. */
final class Instances
{
    private const NAME_MAX_CHARACTERS = 64;

    private const COLUMNS = 'i.id, i.name, i.created_at, i.owner_id, o.username AS owner_username,
            v.id AS version_id, v.created_at AS version_created_at, v.file_count AS version_file_count,
            v.total_bytes AS version_total_bytes, p.username AS version_pusher';

    private const FROM = 'FROM instances i
        JOIN users o ON o.id = i.owner_id
        LEFT JOIN instance_versions v ON v.id = i.latest_version_id
        LEFT JOIN users p ON p.id = v.pushed_by';

    private function __construct()
    {
    }

    /** 1 to 64 characters, valid UTF-8, no control characters. */
    public static function validName(mixed $name): bool
    {
        if (!is_string($name) || preg_match('//u', $name) !== 1) {
            return false;
        }

        $length = mb_strlen($name, 'UTF-8');
        if ($length < 1 || $length > self::NAME_MAX_CHARACTERS) {
            return false;
        }

        return preg_match('/\p{Cc}/u', $name) !== 1;
    }

    /**
     * @return array<string, mixed> The instance as the API returns it, with the owner as the role.
     */
    public static function create(int $ownerId, string $name): array
    {
        $pdo = Database::pdo();
        $statement = $pdo->prepare('INSERT INTO instances (owner_id, name) VALUES (?, ?)');
        $statement->execute([$ownerId, $name]);

        $row = self::row((int) $pdo->lastInsertId());
        if ($row === null) {
            throw new AuthException('not_found', 404);
        }

        return self::present($row, Access::OWNER);
    }

    /**
     * The instances the user owns (role owner) and those shared with the user as editor or viewer, oldest first.
     *
     * @return list<array<string, mixed>>
     */
    public static function listFor(int $userId): array
    {
        $statement = Database::pdo()->prepare(
            'SELECT ' . self::COLUMNS . ', s.access AS share_access ' . self::FROM . '
             LEFT JOIN instance_shares s ON s.instance_id = i.id AND s.friend_user_id = ?
             WHERE i.owner_id = ? OR s.access IN (\'editor\', \'viewer\')
             ORDER BY i.id'
        );
        $statement->execute([$userId, $userId]);

        $instances = [];
        foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
            $role = (int) $row['owner_id'] === $userId ? Access::OWNER : (string) $row['share_access'];
            $instances[] = self::present($row, $role);
        }

        return $instances;
    }

    /**
     * @return array<string, mixed>|null The instance with the given role, or null when it does not exist.
     */
    public static function find(int $instanceId, string $role): ?array
    {
        $row = self::row($instanceId);

        return $row === null ? null : self::present($row, $role);
    }

    /** @throws AuthException not_found (404) when the instance does not exist */
    public static function ownerId(int $instanceId): int
    {
        $statement = Database::pdo()->prepare('SELECT owner_id FROM instances WHERE id = ?');
        $statement->execute([$instanceId]);
        $ownerId = $statement->fetchColumn();

        if ($ownerId === false) {
            throw new AuthException('not_found', 404);
        }

        return (int) $ownerId;
    }

    /** Deletes the instance rows. Its blob objects stay in storage for a later cleanup. */
    public static function delete(int $instanceId): void
    {
        $statement = Database::pdo()->prepare('DELETE FROM instances WHERE id = ?');
        $statement->execute([$instanceId]);
    }

    /** @return array<string, mixed>|null */
    private static function row(int $instanceId): ?array
    {
        $statement = Database::pdo()->prepare('SELECT ' . self::COLUMNS . ' ' . self::FROM . ' WHERE i.id = ?');
        $statement->execute([$instanceId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private static function present(array $row, string $role): array
    {
        $latest = null;
        if ($row['version_id'] !== null) {
            $latest = Versions::summary(
                (int) $row['version_id'],
                (string) $row['version_created_at'],
                (int) $row['version_file_count'],
                (int) $row['version_total_bytes'],
                $row['version_pusher'] === null ? null : (string) $row['version_pusher'],
            );
        }

        return [
            'id' => (int) $row['id'],
            'name' => (string) $row['name'],
            'role' => $role,
            'owner' => ['username' => $row['owner_username'] === null ? null : (string) $row['owner_username']],
            'latest_version' => $latest,
            'created_at' => Times::iso((string) $row['created_at']),
        ];
    }
}
