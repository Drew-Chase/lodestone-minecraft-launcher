<?php

declare(strict_types=1);

namespace tests\Sync;

use app\Users\UserRepository;

/**
 * Rows for the sync tests, written straight to the tables so a test controls every value. Used with DatabaseTestCase.
 */
trait SyncFixtures
{
    /** A verified (or not) account, optionally named and given a paid plan. */
    protected function member(string $email, ?string $username, bool $verified = true, ?string $plan = null): int
    {
        $users = new UserRepository();
        $id = $users->create($email, null, null);

        if ($username !== null) {
            $users->setUsername($id, $username);
        }

        if ($verified) {
            $users->markVerified($id);
        }

        if ($plan !== null) {
            $statement = $this->pdo()->prepare(
                "INSERT INTO subscriptions (user_id, plan, billing_interval, status) VALUES (?, ?, 'monthly', 'active')"
            );
            $statement->execute([$id, $plan]);
        }

        return $id;
    }

    protected function instance(int $ownerId, string $name = 'Survival'): int
    {
        $statement = $this->pdo()->prepare('INSERT INTO instances (owner_id, name) VALUES (?, ?)');
        $statement->execute([$ownerId, $name]);

        return (int) $this->pdo()->lastInsertId();
    }

    protected function share(int $instanceId, int $friendId, string $access): void
    {
        $statement = $this->pdo()->prepare(
            'INSERT INTO instance_shares (instance_id, friend_user_id, access) VALUES (?, ?, ?)'
        );
        $statement->execute([$instanceId, $friendId, $access]);
    }

    protected function blob(string $sha256, int $size): void
    {
        $statement = $this->pdo()->prepare(
            'INSERT INTO blobs (sha256, size) VALUES (?, ?) ON DUPLICATE KEY UPDATE size = size'
        );
        $statement->execute([$sha256, $size]);
    }

    /**
     * A version written directly, with its blobs. The instance's latest version moves to it.
     *
     * @param list<array{path: string, sha256: string, size: int}> $files
     */
    protected function insertVersion(int $instanceId, int $pushedBy, array $files): int
    {
        $pdo = $this->pdo();
        $total = 0;

        foreach ($files as $file) {
            $this->blob($file['sha256'], $file['size']);
            $total += $file['size'];
        }

        $version = $pdo->prepare(
            'INSERT INTO instance_versions (instance_id, pushed_by, file_count, total_bytes) VALUES (?, ?, ?, ?)'
        );
        $version->execute([$instanceId, $pushedBy, count($files), $total]);
        $versionId = (int) $pdo->lastInsertId();

        $row = $pdo->prepare('INSERT INTO version_files (version_id, path, sha256, size, mtime_ms) VALUES (?, ?, ?, ?, 1)');
        foreach ($files as $file) {
            $row->execute([$versionId, $file['path'], $file['sha256'], $file['size']]);
        }

        $pdo->prepare('UPDATE instances SET latest_version_id = ? WHERE id = ?')->execute([$versionId, $instanceId]);

        return $versionId;
    }

    /** A deterministic 64-character lowercase hex digest for a test value. */
    protected function digest(string $seed): string
    {
        return hash('sha256', $seed);
    }
}
