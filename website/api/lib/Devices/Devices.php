<?php

declare(strict_types=1);

namespace app\Devices;

use app\Auth\AuthException;
use app\Auth\RefreshTokens;
use app\Billing\Entitlements;
use app\Db\Database;
use PDO;

/**
 * A user's launcher installations. Revoking one ends every refresh family that was issued to it.
 */
final class Devices
{
    /**
     * The user's active devices, newest first.
     *
     * @return list<array{id: int, name: string, os: string, app_version: string, created_at: string, last_seen_at: ?string}>
     */
    public function active(int $userId): array
    {
        $statement = Database::pdo()->prepare(
            'SELECT id, name, os, app_version, created_at, last_seen_at FROM devices
             WHERE user_id = ? AND revoked_at IS NULL ORDER BY id DESC'
        );
        $statement->execute([$userId]);

        $devices = [];
        foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
            $devices[] = [
                'id' => (int) $row['id'],
                'name' => (string) $row['name'],
                'os' => (string) $row['os'],
                'app_version' => (string) $row['app_version'],
                'created_at' => Times::iso((string) $row['created_at']),
                'last_seen_at' => Times::iso($row['last_seen_at'] === null ? null : (string) $row['last_seen_at']),
            ];
        }

        return $devices;
    }

    /** The device allowance of the user's plan. Null means unlimited. */
    public function limit(int $userId): ?int
    {
        return Entitlements::for($userId)['devices'];
    }

    /**
     * Runs on the connection that is writing the device, so the count and the insert see the same state.
     *
     * @throws AuthException device_limit_reached (409) when the user already has as many active devices as the plan allows
     */
    public function assertRoom(int $userId): void
    {
        $limit = $this->limit($userId);
        if ($limit === null) {
            return;
        }

        $statement = Database::pdo()->prepare(
            'SELECT COUNT(*) FROM devices WHERE user_id = ? AND revoked_at IS NULL'
        );
        $statement->execute([$userId]);

        if ((int) $statement->fetchColumn() >= $limit) {
            throw new AuthException('device_limit_reached', 409);
        }
    }

    public function add(int $userId, string $name, string $os, string $appVersion): int
    {
        $pdo = Database::pdo();
        $statement = $pdo->prepare(
            'INSERT INTO devices (user_id, name, os, app_version, created_at, last_seen_at)
             VALUES (?, ?, ?, ?, NOW(6), NOW(6))'
        );
        $statement->execute([$userId, $name, $os, $appVersion]);

        return (int) $pdo->lastInsertId();
    }

    public function touch(int $deviceId): void
    {
        $statement = Database::pdo()->prepare(
            'UPDATE devices SET last_seen_at = NOW(6) WHERE id = ? AND revoked_at IS NULL'
        );
        $statement->execute([$deviceId]);
    }

    /**
     * Ends the device and every refresh family issued to it. Access tokens already issued stay valid until they expire.
     *
     * @throws AuthException not_found (404) when the device is not one of the user's active devices
     */
    public function revoke(int $userId, int $deviceId): void
    {
        Database::transaction(function (PDO $pdo) use ($userId, $deviceId): void {
            // Checked without a lock, so a device that is not an active device of the user changes nothing.
            $check = $pdo->prepare('SELECT 1 FROM devices WHERE id = ? AND user_id = ? AND revoked_at IS NULL');
            $check->execute([$deviceId, $userId]);
            if ($check->fetchColumn() === false) {
                throw new AuthException('not_found', 404);
            }

            // Token rows are locked before the device row, the order rotation and logout use, so the two cannot deadlock.
            (new RefreshTokens())->revokeForDevice($deviceId);

            $statement = $pdo->prepare(
                'UPDATE devices SET revoked_at = NOW(6) WHERE id = ? AND user_id = ? AND revoked_at IS NULL'
            );
            $statement->execute([$deviceId, $userId]);

            // The device was ended by another request after the check. The rollback also undoes the token revocation.
            if ($statement->rowCount() !== 1) {
                throw new AuthException('not_found', 404);
            }
        });
    }

    /**
     * Marks the device that a refresh family was issued to as revoked, so the list stops showing a launcher that can
     * no longer refresh. A web family has no device and matches nothing. Runs on the connection that revokes the family.
     */
    public function revokeForFamily(string $familyId): void
    {
        $statement = Database::pdo()->prepare(
            'UPDATE devices d
             INNER JOIN refresh_tokens r ON r.device_id = d.id
             SET d.revoked_at = NOW(6)
             WHERE r.family_id = ? AND d.revoked_at IS NULL'
        );
        $statement->execute([$familyId]);
    }

    /**
     * The bulk form of revokeForFamily: marks the user's devices whose live refresh families are being revoked, except
     * the family named. Call it inside a transaction, before the families are revoked, since it matches only unrevoked
     * tokens.
     *
     * The matching token rows are locked first (FOR UPDATE), then the devices. The family revocation that follows in
     * the same transaction reuses those token locks, so the order is tokens, then devices, as everywhere else.
     */
    public function revokeForUserFamilies(int $userId, ?string $exceptFamilyId = null): void
    {
        $pdo = Database::pdo();

        $sql = 'SELECT device_id FROM refresh_tokens WHERE user_id = ? AND revoked_at IS NULL';
        $params = [$userId];

        if ($exceptFamilyId !== null) {
            $sql .= ' AND family_id <> ?';
            $params[] = $exceptFamilyId;
        }

        $select = $pdo->prepare($sql . ' FOR UPDATE');
        $select->execute($params);
        $deviceIds = array_values(array_unique(array_map(
            'intval',
            array_filter($select->fetchAll(PDO::FETCH_COLUMN), static fn (mixed $id): bool => $id !== null),
        )));

        if ($deviceIds === []) {
            return;
        }

        $placeholders = implode(', ', array_fill(0, count($deviceIds), '?'));
        $statement = $pdo->prepare(
            "UPDATE devices SET revoked_at = NOW(6)
             WHERE user_id = ? AND revoked_at IS NULL AND id IN ($placeholders)"
        );
        $statement->execute([$userId, ...$deviceIds]);
    }
}
