<?php

declare(strict_types=1);

namespace app\Auth;

use app\Db\Database;
use app\Devices\Devices;
use PDO;

/**
 * Refresh tokens. Each login starts a family. Every use rotates the token within its family,
 * and a used token coming back revokes the whole family.
 *
 * Every revocation here also marks the device of the family it ends, so a device that can no longer refresh is not
 * listed. The bulk revocations run in one transaction. They lock the token rows, then mark the devices, then revoke the
 * tokens, so the devices are matched while their tokens are still unrevoked.
 *
 * Expiry comes from the database clock (NOW(6) and SQL intervals), not from PHP time.
 */
final class RefreshTokens
{
    public const IDLE_SECONDS = 2592000;      // 30 days without use
    public const ABSOLUTE_SECONDS = 7776000;  // 90 days from sign-in

    /**
     * Starts a new family. A launcher family names the device it was issued to.
     *
     * @return array{token: string, family_id: string}
     */
    public function issue(int $userId, string $client, ?int $deviceId = null): array
    {
        $token = Tokens::random();
        $familyId = Tokens::familyId();

        $statement = Database::pdo()->prepare(
            'INSERT INTO refresh_tokens (user_id, family_id, client, token_hash, idle_expires_at, family_expires_at, device_id)
             VALUES (?, ?, ?, ?, DATE_ADD(NOW(6), INTERVAL ' . self::IDLE_SECONDS . ' SECOND),
                     DATE_ADD(NOW(6), INTERVAL ' . self::ABSOLUTE_SECONDS . ' SECOND), ?)'
        );
        $statement->execute([$userId, $familyId, $client, Tokens::hash($token), $deviceId]);

        return ['token' => $token, 'family_id' => $familyId];
    }

    /**
     * Spends a refresh token and returns its replacement.
     *
     * A token used within the last 10 seconds is refused as refresh_invalid, and nothing is revoked. A token used
     * earlier than that is reuse: its whole family is revoked and refresh_reused is thrown.
     *
     * Revocations must survive, so they are committed rather than thrown from inside the transaction:
     * the callback returns an outcome, and the matching exception is thrown after the commit.
     *
     * @return array{user_id: int, family_id: string, token: string, device_id: ?int}
     * @throws AuthException refresh_invalid, refresh_reused or refresh_expired (401)
     */
    public function rotate(string $presentedToken, string $client, string $ip): array
    {
        if (!Tokens::isWellFormed($presentedToken)) {
            throw new AuthException('refresh_invalid', 401);
        }

        $hash = Tokens::hash($presentedToken);
        $newToken = Tokens::random();
        $idle = self::IDLE_SECONDS;

        /** @var array{status: string, user_id?: int, family_id?: string, device_id?: ?int} $outcome */
        $outcome = Database::transaction(
            function (PDO $pdo) use ($hash, $client, $newToken, $idle, $ip): array {
                $select = $pdo->prepare(
                    'SELECT id, user_id, family_id, client, device_id, used_at, revoked_at, family_expires_at,
                            (NOW(6) >= family_expires_at) AS family_expired,
                            (NOW(6) >= idle_expires_at) AS idle_expired,
                            (TIMESTAMPDIFF(MICROSECOND, used_at, NOW(6)) <= 10000000) AS within_grace
                     FROM refresh_tokens
                     WHERE token_hash = ?
                     FOR UPDATE'
                );
                $select->execute([$hash]);
                $row = $select->fetch(PDO::FETCH_ASSOC);

                if ($row === false || $row['client'] !== $client || $row['revoked_at'] !== null) {
                    return ['status' => 'invalid'];
                }

                $userId = (int) $row['user_id'];
                $familyId = (string) $row['family_id'];

                if ($row['used_at'] !== null) {
                    // Two tabs that refresh at the same moment both present the same token. The second one is refused
                    // without revoking anything, so neither tab signs the other out.
                    if ((int) $row['within_grace'] === 1) {
                        return ['status' => 'grace'];
                    }

                    $this->revokeFamilyOn($pdo, $familyId);
                    AuditLog::recordOn($pdo, $userId, 'refresh_reuse', $ip, null);

                    return ['status' => 'reused', 'user_id' => $userId, 'family_id' => $familyId];
                }

                if ((int) $row['family_expired'] === 1 || (int) $row['idle_expired'] === 1) {
                    $this->revokeFamilyOn($pdo, $familyId);

                    return ['status' => 'expired'];
                }

                $markUsed = $pdo->prepare('UPDATE refresh_tokens SET used_at = NOW(6) WHERE id = ?');
                $markUsed->execute([(int) $row['id']]);

                $insert = $pdo->prepare(
                    'INSERT INTO refresh_tokens (user_id, family_id, client, token_hash, idle_expires_at, family_expires_at, device_id)
                     VALUES (?, ?, ?, ?, DATE_ADD(NOW(6), INTERVAL ' . $idle . ' SECOND), ?, ?)'
                );
                $insert->execute([
                    $userId, $familyId, $client, Tokens::hash($newToken), $row['family_expires_at'], $row['device_id'],
                ]);

                return [
                    'status' => 'ok',
                    'user_id' => $userId,
                    'family_id' => $familyId,
                    'device_id' => $row['device_id'] === null ? null : (int) $row['device_id'],
                ];
            }
        );

        return match ($outcome['status']) {
            'ok' => [
                'user_id' => (int) $outcome['user_id'],
                'family_id' => (string) $outcome['family_id'],
                'token' => $newToken,
                'device_id' => $outcome['device_id'] ?? null,
            ],
            'grace' => throw new AuthException('refresh_invalid', 401),
            'reused' => throw new AuthException('refresh_reused', 401),
            'expired' => throw new AuthException('refresh_expired', 401),
            default => throw new AuthException('refresh_invalid', 401),
        };
    }

    /**
     * Revokes the family of the presented token, whatever its state. Never throws for an unknown or malformed token.
     *
     * @return array{user_id: int, family_id: string}|null
     */
    public function revokeByToken(string $presentedToken): ?array
    {
        if (!Tokens::isWellFormed($presentedToken)) {
            return null;
        }

        $hash = Tokens::hash($presentedToken);

        return Database::transaction(function (PDO $pdo) use ($hash): ?array {
            $select = $pdo->prepare('SELECT user_id, family_id FROM refresh_tokens WHERE token_hash = ?');
            $select->execute([$hash]);
            $row = $select->fetch(PDO::FETCH_ASSOC);

            if ($row === false) {
                return null;
            }

            $familyId = (string) $row['family_id'];
            $this->revokeFamilyOn($pdo, $familyId);

            return ['user_id' => (int) $row['user_id'], 'family_id' => $familyId];
        });
    }

    public function revokeFamily(string $familyId): void
    {
        Database::atomic(function (PDO $pdo) use ($familyId): void {
            $this->revokeFamilyOn($pdo, $familyId);
        });
    }

    /** Revokes every family whose tokens carry the device. Rotation copies the device id, so a family is covered whole. */
    public function revokeForDevice(int $deviceId): void
    {
        $statement = Database::pdo()->prepare(
            'UPDATE refresh_tokens SET revoked_at = NOW(6) WHERE device_id = ? AND revoked_at IS NULL'
        );
        $statement->execute([$deviceId]);
    }

    public function revokeAllForUser(int $userId): void
    {
        Database::atomic(function (PDO $pdo) use ($userId): void {
            (new Devices())->revokeForUserFamilies($userId);

            $statement = $pdo->prepare(
                'UPDATE refresh_tokens SET revoked_at = NOW(6) WHERE user_id = ? AND revoked_at IS NULL'
            );
            $statement->execute([$userId]);
        });
    }

    public function revokeAllExceptFamily(int $userId, string $familyId): void
    {
        Database::atomic(function (PDO $pdo) use ($userId, $familyId): void {
            (new Devices())->revokeForUserFamilies($userId, $familyId);

            $statement = $pdo->prepare(
                'UPDATE refresh_tokens SET revoked_at = NOW(6)
                 WHERE user_id = ? AND family_id <> ? AND revoked_at IS NULL'
            );
            $statement->execute([$userId, $familyId]);
        });
    }

    /**
     * The user's active sessions, one row per family, newest first. Only tokens that are unrevoked and unexpired count.
     *
     * @return list<array{family_id: string, client: string, created_at: string, last_used_at: string}>
     */
    public function listActiveForUser(int $userId): array
    {
        $statement = Database::pdo()->prepare(
            // Every token in a family carries the sign-in time plus the absolute lifetime, so the sign-in time is recovered exactly.
            'SELECT family_id, client,
                    DATE_SUB(MAX(family_expires_at), INTERVAL ' . self::ABSOLUTE_SECONDS . ' SECOND) AS created_at,
                    MAX(COALESCE(used_at, created_at)) AS last_used_at
             FROM refresh_tokens
             WHERE user_id = ? AND revoked_at IS NULL AND idle_expires_at > NOW(6) AND family_expires_at > NOW(6)
             GROUP BY family_id, client
             ORDER BY created_at DESC'
        );
        $statement->execute([$userId]);

        $rows = [];
        foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
            $rows[] = [
                'family_id' => (string) $row['family_id'],
                'client' => (string) $row['client'],
                'created_at' => (string) $row['created_at'],
                'last_used_at' => (string) $row['last_used_at'],
            ];
        }

        return $rows;
    }

    public function familyIsActiveForUser(int $userId, string $familyId): bool
    {
        $statement = Database::pdo()->prepare(
            'SELECT 1 FROM refresh_tokens
             WHERE user_id = ? AND family_id = ? AND revoked_at IS NULL
               AND idle_expires_at > NOW(6) AND family_expires_at > NOW(6)
             LIMIT 1'
        );
        $statement->execute([$userId, $familyId]);

        return $statement->fetchColumn() !== false;
    }

    /** Revokes one family of the user. Returns false when nothing changed, so the family was not theirs or already revoked. */
    public function revokeFamilyForUser(int $userId, string $familyId): bool
    {
        return Database::atomic(function (PDO $pdo) use ($userId, $familyId): bool {
            $statement = $pdo->prepare(
                'UPDATE refresh_tokens SET revoked_at = NOW(6)
                 WHERE user_id = ? AND family_id = ? AND revoked_at IS NULL'
            );
            $statement->execute([$userId, $familyId]);

            if ($statement->rowCount() === 0) {
                return false;
            }

            (new Devices())->revokeForFamily($familyId);

            return true;
        });
    }

    /** The number of the user's active families other than the given one. */
    public function countOtherActive(int $userId, string $familyId): int
    {
        $statement = Database::pdo()->prepare(
            'SELECT COUNT(DISTINCT family_id) FROM refresh_tokens
             WHERE user_id = ? AND family_id <> ? AND revoked_at IS NULL
               AND idle_expires_at > NOW(6) AND family_expires_at > NOW(6)'
        );
        $statement->execute([$userId, $familyId]);

        return (int) $statement->fetchColumn();
    }

    private function revokeFamilyOn(PDO $pdo, string $familyId): void
    {
        $statement = $pdo->prepare(
            'UPDATE refresh_tokens SET revoked_at = NOW(6) WHERE family_id = ? AND revoked_at IS NULL'
        );
        $statement->execute([$familyId]);

        // Database::pdo() is the connection the caller's transaction runs on, so the device is marked in the same commit.
        (new Devices())->revokeForFamily($familyId);
    }
}
