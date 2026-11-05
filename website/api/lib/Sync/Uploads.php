<?php

declare(strict_types=1);

namespace app\Sync;

use app\Auth\AuthException;
use app\Db\Database;
use app\Storage\BlobKeys;
use app\Storage\BlobStore;
use PDO;

/**
 * Upload sessions: a client begins one with its manifest, PUTs the blobs the owner does not already hold, then commits.
 * A commit writes the version only when every blob it needs is proven stored by this session.
 */
final class Uploads
{
    public const SESSION_SECONDS = 3600;
    public const URL_TTL_SECONDS = 900;

    private const CHUNK = 500;

    private function __construct()
    {
    }

    /**
     * Starts a session for a validated manifest. A blob the owner's versions already hold needs no upload. Every other blob
     * gets a presigned PUT, even when the store already has its bytes, since a stored object does not show who wrote it.
     *
     * @param list<array{path: string, sha256: string, size: int, mtime_ms: int}> $files
     * @return array{upload_id: string, expires_at: string, reused: int, uploads: list<array<string, mixed>>}
     * @throws AuthException invalid_device (400), invalid_manifest (400), quota_exceeded (409)
     */
    public static function begin(int $userId, int $instanceId, array $files, ?int $deviceId, BlobStore $store): array
    {
        $ownerId = Instances::ownerId($instanceId);

        if ($deviceId !== null) {
            self::assertDevice($userId, $deviceId);
        }

        $blobs = Manifest::blobs($files);
        $owned = Blobs::referencedBy($ownerId, array_keys($blobs));

        $missing = [];
        $neededBytes = 0;
        foreach ($blobs as $sha256 => $size) {
            if (isset($owned[$sha256])) {
                // The owner's versions hold these bytes at one size, so a different size means a bad manifest.
                if ($owned[$sha256] !== $size) {
                    throw new AuthException('invalid_manifest', 400);
                }

                continue;
            }

            $missing[$sha256] = $size;
            $neededBytes += $size;
        }

        Quota::assertRoom($ownerId, $neededBytes);

        $id = bin2hex(random_bytes(16));

        // Presigned before the session is written, so a failure here leaves no open session behind. Each PUT carries the
        // session id as its tag, which is how a commit tells this session's write from any other write of the same blob.
        $uploads = [];
        foreach ($missing as $sha256 => $size) {
            $signed = $store->presignPut(
                BlobKeys::forSha256($sha256),
                $size,
                BlobKeys::base64Digest($sha256),
                $id,
                self::URL_TTL_SECONDS,
            );

            $uploads[] = [
                'sha256' => $sha256,
                'size' => $size,
                'url' => $signed['url'],
                'headers' => $signed['headers'],
            ];
        }

        $pdo = Database::pdo();

        $insert = $pdo->prepare(
            'INSERT INTO upload_sessions
                (id, instance_id, user_id, device_id, manifest, file_count, total_bytes, status, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, DATE_ADD(NOW(6), INTERVAL ' . self::SESSION_SECONDS . ' SECOND))'
        );
        $insert->execute([
            $id,
            $instanceId,
            $userId,
            $deviceId,
            json_encode($files, JSON_THROW_ON_ERROR),
            count($files),
            Manifest::totalBytes($files),
            'open',
        ]);

        $expires = $pdo->prepare('SELECT expires_at FROM upload_sessions WHERE id = ?');
        $expires->execute([$id]);

        return [
            'upload_id' => $id,
            'expires_at' => Times::iso((string) $expires->fetchColumn()),
            'reused' => count($owned),
            'uploads' => $uploads,
        ];
    }

    /**
     * Commits a session. Returns ['missing' => [sha256, ...]] when a blob is not proven stored by this session, and leaves
     * the session open. Otherwise returns ['version' => ...] after writing the version.
     *
     * @return array{missing: list<string>}|array{version: array<string, mixed>}
     * @throws AuthException not_found (404), upload_expired (410), upload_committed (409), quota_exceeded (409),
     *                       invalid_manifest (400)
     */
    public static function commit(int $userId, int $instanceId, string $uploadId, BlobStore $store): array
    {
        $statement = Database::pdo()->prepare(
            'SELECT user_id, status, manifest, (expires_at <= NOW(6)) AS expired
             FROM upload_sessions WHERE id = ? AND instance_id = ?'
        );
        $statement->execute([$uploadId, $instanceId]);
        $session = $statement->fetch(PDO::FETCH_ASSOC);

        // A session another user started is reported as absent, so its id reveals nothing.
        if ($session === false || (int) $session['user_id'] !== $userId) {
            throw new AuthException('not_found', 404);
        }
        self::assertOpen($session);

        $files = self::manifest((string) $session['manifest']);
        $blobs = Manifest::blobs($files);

        $missing = self::missing(Instances::ownerId($instanceId), $uploadId, $blobs, $store);
        if ($missing !== []) {
            return ['missing' => $missing];
        }

        $version = Database::transaction(
            static fn (PDO $pdo): array => self::record($pdo, $userId, $instanceId, $uploadId, $files, $blobs)
        );

        return ['version' => $version];
    }

    /**
     * Writes the version inside the caller's transaction. The instance row, then the owner, then the session are locked,
     * always in that order, so commits for one owner run one at a time and the quota check sees the latest state.
     *
     * @param list<array{path: string, sha256: string, size: int, mtime_ms: int}> $files
     * @param array<string, int> $blobs
     * @return array<string, mixed>
     * @throws AuthException invalid_manifest (400) when a stored blob has another size, quota_exceeded (409)
     */
    private static function record(
        PDO $pdo,
        int $userId,
        int $instanceId,
        string $uploadId,
        array $files,
        array $blobs,
    ): array {
        $owner = $pdo->prepare('SELECT owner_id FROM instances WHERE id = ? FOR UPDATE');
        $owner->execute([$instanceId]);
        $ownerId = $owner->fetchColumn();
        if ($ownerId === false) {
            throw new AuthException('not_found', 404);
        }

        $pdo->prepare('SELECT id FROM users WHERE id = ? FOR UPDATE')->execute([(int) $ownerId]);

        $lock = $pdo->prepare(
            'SELECT device_id, status, (expires_at <= NOW(6)) AS expired
             FROM upload_sessions WHERE id = ? AND instance_id = ? FOR UPDATE'
        );
        $lock->execute([$uploadId, $instanceId]);
        $session = $lock->fetch(PDO::FETCH_ASSOC);
        if ($session === false) {
            throw new AuthException('not_found', 404);
        }
        self::assertOpen($session);

        $stored = Blobs::storedSizes(array_keys($blobs));
        $insertBlob = $pdo->prepare('INSERT INTO blobs (sha256, size) VALUES (?, ?) ON DUPLICATE KEY UPDATE size = size');
        foreach ($blobs as $sha256 => $size) {
            if (!isset($stored[$sha256])) {
                $insertBlob->execute([$sha256, $size]);
            } elseif ($stored[$sha256] !== $size) {
                // A digest names one set of bytes, so a manifest that disagrees with the table is refused.
                throw new AuthException('invalid_manifest', 400);
            }
        }

        $pdo->prepare(
            'INSERT INTO instance_versions (instance_id, pushed_by, device_id, file_count, total_bytes)
             VALUES (?, ?, ?, ?, ?)'
        )->execute([
            $instanceId,
            $userId,
            $session['device_id'],
            count($files),
            Manifest::totalBytes($files),
        ]);
        $versionId = (int) $pdo->lastInsertId();

        foreach (array_chunk($files, self::CHUNK) as $chunk) {
            $placeholders = implode(', ', array_fill(0, count($chunk), '(?, ?, ?, ?, ?)'));
            $params = [];
            foreach ($chunk as $file) {
                array_push($params, $versionId, $file['path'], $file['sha256'], $file['size'], $file['mtime_ms']);
            }

            $pdo->prepare(
                "INSERT INTO version_files (version_id, path, sha256, size, mtime_ms) VALUES $placeholders"
            )->execute($params);
        }

        // Checked once the rows are in, so the total counts every blob this version newly holds, including one another
        // user's upload put in the store. An excess rolls the whole commit back.
        Quota::assertRoom((int) $ownerId, 0);

        $pdo->prepare('UPDATE instances SET latest_version_id = ? WHERE id = ?')->execute([$versionId, $instanceId]);

        // The conditional update is the final guard: if it changes no row, another commit has already finished.
        $close = $pdo->prepare("UPDATE upload_sessions SET status = 'committed' WHERE id = ? AND status = 'open'");
        $close->execute([$uploadId]);
        if ($close->rowCount() !== 1) {
            throw new AuthException('upload_committed', 409);
        }

        return Versions::find($versionId);
    }

    /**
     * The blobs this commit needs that are not proven stored by this session. A blob the owner's versions already hold is
     * proven. Any other blob must be an object whose checksum and size match and whose last write carried this session's tag.
     * The tag is what proves the client had the bytes: an object another user wrote has a different tag, however well it matches.
     *
     * @param array<string, int> $blobs
     * @return list<string>
     */
    private static function missing(int $ownerId, string $uploadId, array $blobs, BlobStore $store): array
    {
        $owned = Blobs::referencedBy($ownerId, array_keys($blobs));

        $missing = [];
        foreach ($blobs as $sha256 => $size) {
            if (isset($owned[$sha256])) {
                continue;
            }

            $head = $store->head(BlobKeys::forSha256($sha256));

            // A missing checksum or tag counts as a mismatch, since the object cannot be shown to match.
            if (
                $head === null
                || $head['size'] !== $size
                || $head['sha256'] !== BlobKeys::base64Digest($sha256)
                || $head['tag'] !== $uploadId
            ) {
                $missing[] = $sha256;
            }
        }

        return $missing;
    }

    /**
     * @param array<string, mixed> $session A row with status and expired (1 when past expires_at).
     * @throws AuthException upload_committed (409) or upload_expired (410)
     */
    private static function assertOpen(array $session): void
    {
        if ((string) $session['status'] === 'committed') {
            throw new AuthException('upload_committed', 409);
        }

        if ((int) $session['expired'] === 1) {
            throw new AuthException('upload_expired', 410);
        }
    }

    /** @return list<array{path: string, sha256: string, size: int, mtime_ms: int}> */
    private static function manifest(string $json): array
    {
        /** @var list<array{path: string, sha256: string, size: int, mtime_ms: int}> $files */
        $files = json_decode($json, true, 512, JSON_THROW_ON_ERROR);

        return $files;
    }

    /** @throws AuthException invalid_device (400) when the device is not an active device of the user */
    private static function assertDevice(int $userId, int $deviceId): void
    {
        $statement = Database::pdo()->prepare(
            'SELECT 1 FROM devices WHERE id = ? AND user_id = ? AND revoked_at IS NULL'
        );
        $statement->execute([$deviceId, $userId]);

        if ($statement->fetchColumn() === false) {
            throw new AuthException('invalid_device', 400);
        }
    }
}
