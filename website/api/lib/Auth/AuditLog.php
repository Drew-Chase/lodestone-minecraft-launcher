<?php

declare(strict_types=1);

namespace app\Auth;

use app\Db\Database;
use PDO;

/**
 * Account and session events in audit_events. Callers pass short details only: never a token, password or hash.
 */
final class AuditLog
{
    private const MAX_DETAIL_BYTES = 255;

    public function record(?int $userId, string $type, ?string $ip, ?string $detail = null): void
    {
        self::recordOn(Database::pdo(), $userId, $type, $ip, $detail);
    }

    /**
     * Records on the given connection, so the row joins that connection's open transaction.
     */
    public static function recordOn(PDO $pdo, ?int $userId, string $type, ?string $ip, ?string $detail = null): void
    {
        // mb_strcut cuts on byte offsets without splitting a UTF-8 character.
        $trimmed = $detail === null ? null : mb_strcut($detail, 0, self::MAX_DETAIL_BYTES, 'UTF-8');

        $statement = $pdo->prepare(
            'INSERT INTO audit_events (user_id, type, ip, detail) VALUES (?, ?, ?, ?)'
        );
        $statement->execute([$userId, $type, $ip, $trimmed]);
    }
}
