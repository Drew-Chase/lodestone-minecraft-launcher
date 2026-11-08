<?php

declare(strict_types=1);

namespace app\Auth;

use app\Db\Database;
use PDO;

/**
 * Fixed-window counters in the rate_limits table. Bucket keys are hashed, so no address or email is stored.
 */
final class RateLimiter
{
    private const KEEP_SECONDS = 86400;

    /** True when the current window's count has reached $limit. */
    public function isBlocked(string $bucket, int $limit, int $windowSeconds): bool
    {
        $statement = Database::pdo()->prepare(
            'SELECT hits FROM rate_limits WHERE bucket_key = ? AND window_start = ?'
        );
        $statement->execute([Tokens::hash($bucket), $this->windowStart($windowSeconds)]);
        $hits = $statement->fetchColumn();

        return $hits !== false && (int) $hits >= $limit;
    }

    /** Adds one hit to the current window and returns the window's count. */
    public function hit(string $bucket, int $windowSeconds): int
    {
        $pdo = Database::pdo();
        $key = Tokens::hash($bucket);
        $start = $this->windowStart($windowSeconds);

        $upsert = $pdo->prepare(
            'INSERT INTO rate_limits (bucket_key, window_start, hits) VALUES (?, ?, 1)
             ON DUPLICATE KEY UPDATE hits = hits + 1'
        );
        $upsert->execute([$key, $start]);

        $select = $pdo->prepare('SELECT hits FROM rate_limits WHERE bucket_key = ? AND window_start = ?');
        $select->execute([$key, $start]);
        $count = (int) $select->fetchColumn();

        // Old windows are swept on about one hit in a hundred, which keeps the table small without a cron job.
        if (random_int(1, 100) === 1) {
            $sweep = $pdo->prepare('DELETE FROM rate_limits WHERE window_start < ?');
            $sweep->execute([time() - self::KEEP_SECONDS]);
        }

        return $count;
    }

    /** Gives back one hit the current window counted, for an attempt that succeeded, so only the failures stay counted. */
    public function release(string $bucket, int $windowSeconds): void
    {
        $statement = Database::pdo()->prepare(
            'UPDATE rate_limits SET hits = hits - 1 WHERE bucket_key = ? AND window_start = ? AND hits > 0'
        );
        $statement->execute([Tokens::hash($bucket), $this->windowStart($windowSeconds)]);
    }

    /** Removes every window of the bucket. */
    public function clear(string $bucket): void
    {
        $statement = Database::pdo()->prepare('DELETE FROM rate_limits WHERE bucket_key = ?');
        $statement->execute([Tokens::hash($bucket)]);
    }

    /** Seconds until the current window ends, at least 1. */
    public function retryAfter(int $windowSeconds): int
    {
        $end = $this->windowStart($windowSeconds) + $windowSeconds;

        return max(1, $end - time());
    }

    private function windowStart(int $windowSeconds): int
    {
        return intdiv(time(), $windowSeconds) * $windowSeconds;
    }
}
