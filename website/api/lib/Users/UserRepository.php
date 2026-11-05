<?php

declare(strict_types=1);

namespace app\Users;

use app\Db\Database;
use DateTimeImmutable;
use DateTimeZone;
use PDO;

/**
 * Reads and writes the users table. Lookups return soft-deleted rows too: callers check deleted_at.
 */
final class UserRepository
{
    /** @return array<string, mixed>|null */
    public function findByEmail(string $email): ?array
    {
        $statement = Database::pdo()->prepare('SELECT * FROM users WHERE email = ?');
        $statement->execute([$email]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /** @return array<string, mixed>|null */
    public function findById(int $id): ?array
    {
        $statement = Database::pdo()->prepare('SELECT * FROM users WHERE id = ?');
        $statement->execute([$id]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /** @return array<string, mixed>|null Matches case-insensitively, because the column uses ascii_general_ci. */
    public function findByUsername(string $username): ?array
    {
        $statement = Database::pdo()->prepare('SELECT * FROM users WHERE username = ?');
        $statement->execute([$username]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /** 3 to 16 ASCII letters, digits or underscores. */
    public static function validUsername(string $username): bool
    {
        return preg_match('/^[A-Za-z0-9_]{3,16}$/D', $username) === 1;
    }

    public function setUsername(int $id, string $username): void
    {
        $statement = Database::pdo()->prepare('UPDATE users SET username = ? WHERE id = ?');
        $statement->execute([$username, $id]);
    }

    public function create(string $email, ?string $passwordHash, ?string $displayName): int
    {
        $statement = Database::pdo()->prepare(
            'INSERT INTO users (email, password_hash, display_name) VALUES (?, ?, ?)'
        );
        $statement->execute([$email, $passwordHash, $displayName]);

        return (int) Database::pdo()->lastInsertId();
    }

    public function markVerified(int $id): void
    {
        $statement = Database::pdo()->prepare(
            'UPDATE users SET email_verified_at = NOW(6) WHERE id = ? AND email_verified_at IS NULL'
        );
        $statement->execute([$id]);
    }

    public function setPasswordHash(int $id, string $hash): void
    {
        $statement = Database::pdo()->prepare('UPDATE users SET password_hash = ? WHERE id = ?');
        $statement->execute([$hash, $id]);
    }

    /**
     * The public shape of a user. Never includes the password hash.
     *
     * @param array<string, mixed> $row
     * @return array{id: int, email: string, email_verified: bool, display_name: ?string, username: ?string, presence_hidden: bool, created_at: string}
     */
    public function toPublic(array $row): array
    {
        $displayName = $row['display_name'] ?? null;
        $username = $row['username'] ?? null;

        return [
            'id' => (int) $row['id'],
            'email' => (string) $row['email'],
            'email_verified' => $row['email_verified_at'] !== null,
            'display_name' => $displayName === null ? null : (string) $displayName,
            'username' => $username === null ? null : (string) $username,
            'presence_hidden' => (int) $row['presence_hidden'] === 1,
            'created_at' => $this->isoUtc((string) $row['created_at']),
        ];
    }

    /**
     * Accepts DATETIME(6) text with or without fractional seconds. The stored times are UTC.
     */
    private function isoUtc(string $databaseTime): string
    {
        $time = new DateTimeImmutable($databaseTime, new DateTimeZone('UTC'));

        return $time->format(DATE_ATOM);
    }
}
