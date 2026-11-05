<?php

declare(strict_types=1);

namespace app\Minecraft;

use app\Db\Database;
use PDO;

/**
 * Reads and writes minecraft_link_sessions. Expiry and poll spacing are compared with the database clock, so the
 * web servers' clocks never decide them.
 */
final class LinkSessions
{
    public function insert(string $id, int $userId, DeviceCode $code): void
    {
        $statement = Database::pdo()->prepare(
            'INSERT INTO minecraft_link_sessions
                (id, user_id, device_code, user_code, verification_uri, interval_seconds, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, DATE_ADD(NOW(6), INTERVAL ' . $code->expiresIn . ' SECOND))'
        );
        $statement->execute([
            $id,
            $userId,
            $code->deviceCode,
            $code->userCode,
            $code->verificationUri,
            $code->interval,
        ]);
    }

    /**
     * The session if the user owns it. `expired` is 1 once the database clock has passed its expiry.
     *
     * @return array<string, mixed>|null
     */
    public function find(string $id, int $userId): ?array
    {
        $statement = Database::pdo()->prepare(
            'SELECT id, device_code, interval_seconds, expires_at, (expires_at <= NOW(6)) AS expired
             FROM minecraft_link_sessions
             WHERE id = ? AND user_id = ?'
        );
        $statement->execute([$id, $userId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /**
     * Stamps the poll time when the session has waited its interval since the last poll. The check and the stamp
     * are one statement, so two polls at once cannot both reach Microsoft.
     *
     * @return bool|null true when the poll may go to Microsoft, false when the interval has not passed yet, and null
     *                   when the user holds no such session, because it is missing or belongs to someone else
     */
    public function claimPoll(string $id, int $userId): ?bool
    {
        $claim = Database::pdo()->prepare(
            'UPDATE minecraft_link_sessions
             SET last_polled_at = NOW(6)
             WHERE id = ? AND user_id = ?
               AND (last_polled_at IS NULL OR last_polled_at <= DATE_SUB(NOW(6), INTERVAL interval_seconds SECOND))'
        );
        $claim->execute([$id, $userId]);
        if ($claim->rowCount() === 1) {
            return true;
        }

        // Nothing was stamped. Only a session that is still there means the interval has not passed.
        $exists = Database::pdo()->prepare('SELECT 1 FROM minecraft_link_sessions WHERE id = ? AND user_id = ?');
        $exists->execute([$id, $userId]);

        return $exists->fetchColumn() === false ? null : false;
    }

    public function setInterval(string $id, int $seconds): void
    {
        $statement = Database::pdo()->prepare('UPDATE minecraft_link_sessions SET interval_seconds = ? WHERE id = ?');
        $statement->execute([$seconds, $id]);
    }

    public function delete(string $id): void
    {
        $statement = Database::pdo()->prepare('DELETE FROM minecraft_link_sessions WHERE id = ?');
        $statement->execute([$id]);
    }

    public function deleteForUser(int $userId): void
    {
        $statement = Database::pdo()->prepare('DELETE FROM minecraft_link_sessions WHERE user_id = ?');
        $statement->execute([$userId]);
    }

    /** Removes sessions nobody polled to the end, so their device codes do not outlive the sign-in. */
    public function deleteExpired(): void
    {
        Database::pdo()->exec('DELETE FROM minecraft_link_sessions WHERE expires_at <= NOW(6)');
    }
}
