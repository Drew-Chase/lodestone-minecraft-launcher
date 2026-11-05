<?php

declare(strict_types=1);

namespace app\Sync;

use app\Auth\AuthException;
use app\Billing\Entitlements;
use app\Db\Database;
use PDO;

/**
 * Storage an owner uses: the distinct blobs that any version of an instance the owner holds refers to. Shares and
 * editors never count against the owner's allowance.
 */
final class Quota
{
    private function __construct()
    {
    }

    public static function usedBytes(int $ownerId): int
    {
        $statement = Database::pdo()->prepare(
            'SELECT COALESCE(SUM(b.size), 0) FROM blobs b
             WHERE b.sha256 IN (
                 SELECT vf.sha256 FROM version_files vf
                 JOIN instance_versions iv ON iv.id = vf.version_id
                 JOIN instances i ON i.id = iv.instance_id
                 WHERE i.owner_id = ?
             )'
        );
        $statement->execute([$ownerId]);

        return (int) $statement->fetchColumn();
    }

    /** The owner's storage allowance in bytes, from the plan. */
    public static function limitBytes(int $ownerId): int
    {
        return Entitlements::for($ownerId)['storage_bytes'];
    }

    /**
     * Refuses when the owner's used bytes plus the new bytes pass the allowance. The caller passes the bytes of the blobs
     * not yet stored.
     *
     * @throws AuthException quota_exceeded (409)
     */
    public static function assertRoom(int $ownerId, int $neededBytes): void
    {
        if (self::usedBytes($ownerId) + $neededBytes > self::limitBytes($ownerId)) {
            throw new AuthException('quota_exceeded', 409);
        }
    }
}
