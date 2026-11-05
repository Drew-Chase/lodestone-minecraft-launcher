<?php

declare(strict_types=1);

namespace app\Auth;

/**
 * Hashes passwords with argon2id when PHP has it, and with bcrypt otherwise.
 *
 * bcrypt reads only the first 72 bytes of its input, so a bcrypt hash is made over the base64 SHA-256 digest of
 * the password. The digest is 44 characters long, so all 256 bytes count. argon2id takes the password as it is.
 */
final class PasswordHasher
{
    private const DUMMY_SOURCE = 'lodestone-burn-time';

    /** @var array<string, string> Dummy hashes by algorithm. Hashed once per process, not once per instance. */
    private static array $dummyHashes = [];

    /** A PASSWORD_* constant is a string in PHP ('2y', 'argon2id'), so the parameter is string|int|null, not ?int. */
    private readonly string|int $algorithm;

    public function __construct(string|int|null $algorithm = null)
    {
        $this->algorithm = $algorithm ?? (defined('PASSWORD_ARGON2ID') ? PASSWORD_ARGON2ID : PASSWORD_BCRYPT);
    }

    public function hash(string $password): string
    {
        $hash = password_hash($this->prepare($password, $this->algorithm), $this->algorithm);
        if ($hash === false) {
            throw new \RuntimeException('Password hashing failed');
        }

        return $hash;
    }

    public function verify(string $password, string $hash): bool
    {
        $algo = password_get_info($hash)['algo'];

        return password_verify($this->prepare($password, $algo), $hash);
    }

    public function needsRehash(string $hash): bool
    {
        if (password_get_info($hash)['algo'] !== $this->algorithm) {
            return true;
        }

        return password_needs_rehash($hash, $this->algorithm);
    }

    /**
     * Runs one verify against a fixed dummy hash, so an unknown address takes about as long as a known one.
     * The dummy is made with the same algorithm as real passwords, and it is cached per algorithm. It is not cached
     * per instance: the service builds a hasher for every request, so a per-instance cache would make the first
     * call in each request pay for a second hash.
     */
    public function burnTime(): void
    {
        $key = (string) $this->algorithm;
        if (!isset(self::$dummyHashes[$key])) {
            self::$dummyHashes[$key] = $this->hash(self::DUMMY_SOURCE);
        }

        $this->verify('burn-time', self::$dummyHashes[$key]);
    }

    /**
     * bcrypt hashes the base64 SHA-256 digest. Every other algorithm takes the raw password.
     */
    private function prepare(string $password, string|int|null $algorithm): string
    {
        return $algorithm === PASSWORD_BCRYPT
            ? base64_encode(hash('sha256', $password, true))
            : $password;
    }
}
