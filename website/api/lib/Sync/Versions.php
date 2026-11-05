<?php

declare(strict_types=1);

namespace app\Sync;

use app\Auth\AuthException;
use app\Db\Database;
use app\Storage\BlobKeys;
use app\Storage\BlobStore;
use PDO;

/** Version history of an instance: the list, the latest version and one version, with presigned downloads. */
final class Versions
{
    public const LIST_LIMIT = 100;
    public const URL_TTL_SECONDS = 900;

    private const SUMMARY = 'SELECT v.id, v.created_at, v.file_count, v.total_bytes, u.username AS pusher
        FROM instance_versions v
        JOIN users u ON u.id = v.pushed_by';

    private function __construct()
    {
    }

    /**
     * The newest versions first, at most LIST_LIMIT of them.
     *
     * @return list<array<string, mixed>>
     */
    public static function list(int $instanceId): array
    {
        $statement = Database::pdo()->prepare(
            self::SUMMARY . ' WHERE v.instance_id = ? ORDER BY v.id DESC LIMIT ' . self::LIST_LIMIT
        );
        $statement->execute([$instanceId]);

        $versions = [];
        foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
            $versions[] = self::fromRow($row);
        }

        return $versions;
    }

    /**
     * The latest version with its files, or null when the instance has no version yet.
     *
     * @return array{version: array<string, mixed>, files: list<array<string, mixed>>}|null
     */
    public static function latest(int $instanceId, BlobStore $store): ?array
    {
        $statement = Database::pdo()->prepare('SELECT latest_version_id FROM instances WHERE id = ?');
        $statement->execute([$instanceId]);
        $latest = $statement->fetchColumn();

        if ($latest === false || $latest === null) {
            return null;
        }

        return self::detail($instanceId, (int) $latest, $store);
    }

    /**
     * @return array{version: array<string, mixed>, files: list<array<string, mixed>>}|null null when the version is not in the instance
     */
    public static function one(int $instanceId, int $versionId, BlobStore $store): ?array
    {
        return self::detail($instanceId, $versionId, $store);
    }

    /**
     * One version as the API returns it, read on the caller's connection. Used right after a version is written.
     *
     * @return array<string, mixed>
     * @throws AuthException not_found (404) when the version does not exist
     */
    public static function find(int $versionId): array
    {
        $statement = Database::pdo()->prepare(self::SUMMARY . ' WHERE v.id = ?');
        $statement->execute([$versionId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        if ($row === false) {
            throw new AuthException('not_found', 404);
        }

        return self::fromRow($row);
    }

    /** @return array<string, mixed> */
    public static function summary(int $id, string $createdAt, int $fileCount, int $totalBytes, ?string $pushedBy): array
    {
        return [
            'id' => $id,
            'created_at' => Times::iso($createdAt),
            'file_count' => $fileCount,
            'total_bytes' => $totalBytes,
            'pushed_by' => $pushedBy,
        ];
    }

    /**
     * @return array{version: array<string, mixed>, files: list<array<string, mixed>>}|null
     */
    private static function detail(int $instanceId, int $versionId, BlobStore $store): ?array
    {
        $statement = Database::pdo()->prepare(self::SUMMARY . ' WHERE v.id = ? AND v.instance_id = ?');
        $statement->execute([$versionId, $instanceId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        if ($row === false) {
            return null;
        }

        $files = Database::pdo()->prepare(
            'SELECT path, sha256, size, mtime_ms FROM version_files WHERE version_id = ? ORDER BY path'
        );
        $files->execute([$versionId]);

        $list = [];
        foreach ($files->fetchAll(PDO::FETCH_ASSOC) as $file) {
            $sha256 = (string) $file['sha256'];
            $list[] = [
                'path' => (string) $file['path'],
                'sha256' => $sha256,
                'size' => (int) $file['size'],
                'mtime_ms' => (int) $file['mtime_ms'],
                'url' => $store->presignGet(
                    BlobKeys::forSha256($sha256),
                    self::URL_TTL_SECONDS,
                    basename((string) $file['path']),
                ),
            ];
        }

        return ['version' => self::fromRow($row), 'files' => $list];
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private static function fromRow(array $row): array
    {
        return self::summary(
            (int) $row['id'],
            (string) $row['created_at'],
            (int) $row['file_count'],
            (int) $row['total_bytes'],
            (string) $row['pusher'],
        );
    }
}
