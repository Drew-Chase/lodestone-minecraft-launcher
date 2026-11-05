<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use app\Db\Database;
use PDO;

/**
 * The single-use records of started sign-ins. Only the SHA-256 of the state is stored, so a database
 * read cannot replay a callback.
 */
final class OAuthStates
{
    private const TTL_SECONDS = 600;

    // An expired state is never used again. Pruning one start in fifty keeps anonymous starts from growing the table.
    private const PRUNE_ONE_IN = 50;
    private const PRUNE_AFTER_SECONDS = 86400;

    /**
     * @return array{state: string, code_challenge: string, verifier: string, nonce: string}
     */
    public function create(string $provider, string $nextPath): array
    {
        $state = Responses::base64Url(random_bytes(32));
        $verifier = Responses::base64Url(random_bytes(32));
        $nonce = Responses::base64Url(random_bytes(16));
        $challenge = Responses::base64Url(hash('sha256', $verifier, true));

        if (random_int(1, self::PRUNE_ONE_IN) === 1) {
            Database::pdo()->exec(
                'DELETE FROM oauth_states WHERE expires_at < DATE_SUB(NOW(6), INTERVAL ' . self::PRUNE_AFTER_SECONDS . ' SECOND)'
            );
        }

        $statement = Database::pdo()->prepare(
            'INSERT INTO oauth_states (state_hash, provider, pkce_verifier, nonce, next_path, expires_at)
             VALUES (?, ?, ?, ?, ?, DATE_ADD(NOW(6), INTERVAL ' . self::TTL_SECONDS . ' SECOND))'
        );
        $statement->execute([hash('sha256', $state), $provider, $verifier, $nonce, $nextPath]);

        return [
            'state' => $state,
            'code_challenge' => $challenge,
            'verifier' => $verifier,
            'nonce' => $nonce,
        ];
    }

    /**
     * Spends the state. Only the first caller changes the row, and an expired or foreign state changes nothing.
     *
     * @return array{pkce_verifier: string, nonce: string, next_path: string}|null
     */
    public function consume(string $state, string $provider): ?array
    {
        $pdo = Database::pdo();
        $hash = hash('sha256', $state);

        $spend = $pdo->prepare(
            'UPDATE oauth_states SET used_at = NOW(6)
             WHERE state_hash = ? AND provider = ? AND used_at IS NULL AND expires_at > NOW(6)'
        );
        $spend->execute([$hash, $provider]);

        if ($spend->rowCount() !== 1) {
            return null;
        }

        $read = $pdo->prepare('SELECT pkce_verifier, nonce, next_path FROM oauth_states WHERE state_hash = ?');
        $read->execute([$hash]);
        $row = $read->fetch(PDO::FETCH_ASSOC);
        if ($row === false) {
            return null;
        }

        return [
            'pkce_verifier' => (string) $row['pkce_verifier'],
            'nonce' => (string) $row['nonce'],
            'next_path' => (string) $row['next_path'],
        ];
    }
}
