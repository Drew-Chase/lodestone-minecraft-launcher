<?php

declare(strict_types=1);

namespace app\Friends;

use app\Auth\AuthException;
use app\Auth\RateLimiter;
use app\Users\UserRepository;

/**
 * The checks a route runs before its work: the account still exists, the email is verified, the username is set,
 * and the rate limit has room.
 */
final class Gate
{
    private function __construct()
    {
    }

    /**
     * @return array<string, mixed>
     * @throws AuthException unauthorized (401) when the account is missing or deleted
     */
    public static function active(int $userId): array
    {
        $user = (new UserRepository())->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            throw new AuthException('unauthorized', 401);
        }

        return $user;
    }

    /** @param array<string, mixed> $user @throws AuthException email_not_verified (403) */
    public static function verified(array $user): void
    {
        if ($user['email_verified_at'] === null) {
            throw new AuthException('email_not_verified', 403);
        }
    }

    /** @param array<string, mixed> $user @throws AuthException username_required (409) */
    public static function named(array $user): void
    {
        if ($user['username'] === null) {
            throw new AuthException('username_required', 409);
        }
    }

    /**
     * Counts one attempt in a fixed window and throws once the count passes the limit.
     *
     * @throws AuthException rate_limited (429)
     */
    public static function limit(string $bucket, int $max, int $windowSeconds): void
    {
        $limiter = new RateLimiter();
        if ($limiter->hit($bucket, $windowSeconds) > $max) {
            throw new AuthException('rate_limited', 429, $limiter->retryAfter($windowSeconds));
        }
    }
}
