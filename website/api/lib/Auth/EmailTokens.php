<?php

declare(strict_types=1);

namespace app\Auth;

use app\Db\Database;
use PDO;

/**
 * Single-use tokens for email links (verify and reset). Only their hashes are stored.
 */
final class EmailTokens
{
    public const VERIFY = 'verify';
    public const RESET = 'reset';

    /**
     * Invalidates earlier unused tokens of the same purpose, then returns a new raw token.
     */
    public function create(int $userId, string $purpose, int $ttlSeconds): string
    {
        $raw = Tokens::random();
        $ttl = max(1, $ttlSeconds);

        Database::transaction(function (PDO $pdo) use ($userId, $purpose, $raw, $ttl): void {
            $invalidate = $pdo->prepare(
                'UPDATE email_tokens SET used_at = NOW(6)
                 WHERE user_id = ? AND purpose = ? AND used_at IS NULL'
            );
            $invalidate->execute([$userId, $purpose]);

            $insert = $pdo->prepare(
                'INSERT INTO email_tokens (user_id, purpose, token_hash, expires_at)
                 VALUES (?, ?, ?, DATE_ADD(NOW(6), INTERVAL ' . $ttl . ' SECOND))'
            );
            $insert->execute([$userId, $purpose, Tokens::hash($raw)]);
        });

        return $raw;
    }

    /**
     * Spends the token and returns the user id it belongs to.
     *
     * @throws AuthException invalid_token (400) or token_expired (400)
     */
    public function consume(string $rawToken, string $purpose): int
    {
        if (!Tokens::isWellFormed($rawToken)) {
            throw new AuthException('invalid_token', 400);
        }

        $hash = Tokens::hash($rawToken);
        $pdo = Database::pdo();

        // One statement decides the race: only the first caller changes the row.
        $consume = $pdo->prepare(
            'UPDATE email_tokens SET used_at = NOW(6)
             WHERE token_hash = ? AND purpose = ? AND used_at IS NULL AND expires_at > NOW(6)'
        );
        $consume->execute([$hash, $purpose]);

        if ($consume->rowCount() === 1) {
            $select = $pdo->prepare('SELECT user_id FROM email_tokens WHERE token_hash = ?');
            $select->execute([$hash]);

            return (int) $select->fetchColumn();
        }

        $inspect = $pdo->prepare(
            'SELECT purpose, used_at, (expires_at <= NOW(6)) AS expired
             FROM email_tokens WHERE token_hash = ?'
        );
        $inspect->execute([$hash]);
        $row = $inspect->fetch(PDO::FETCH_ASSOC);

        if ($row === false || $row['purpose'] !== $purpose || $row['used_at'] !== null) {
            throw new AuthException('invalid_token', 400);
        }

        if ((int) $row['expired'] === 1) {
            throw new AuthException('token_expired', 400);
        }

        // The row changed between the update and this read, so the token is no longer usable.
        throw new AuthException('invalid_token', 400);
    }
}
