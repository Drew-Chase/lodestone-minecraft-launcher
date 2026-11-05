<?php

declare(strict_types=1);

namespace app\Auth;

/**
 * Random opaque tokens. Only their SHA-256 hashes are stored.
 */
final class Tokens
{
    private function __construct()
    {
    }

    /** 64 lowercase hex characters from 32 random bytes. */
    public static function random(): string
    {
        return bin2hex(random_bytes(32));
    }

    /** SHA-256 of the token, lowercase hex. This is what the tables store. */
    public static function hash(string $token): string
    {
        return hash('sha256', $token);
    }

    public static function isWellFormed(string $token): bool
    {
        return preg_match('/^[a-f0-9]{64}$/D', $token) === 1;
    }

    /** 32 lowercase hex characters identifying one login session. */
    public static function familyId(): string
    {
        return bin2hex(random_bytes(16));
    }
}
