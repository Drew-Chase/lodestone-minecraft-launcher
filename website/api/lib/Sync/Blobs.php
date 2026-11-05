<?php

declare(strict_types=1);

namespace app\Sync;

use app\Db\Database;
use PDO;

/** Reads the blobs table, which lists the objects known to be stored and verified. */
final class Blobs
{
    /** Keeps each IN list well under the server's placeholder limit. */
    private const CHUNK = 500;

    private function __construct()
    {
    }

    /**
     * The sizes of the given blobs that some version of an instance the owner holds refers to. A blob is here only once a
     * commit by that owner has proven its bytes, so a hash a user merely knows does not count.
     *
     * @param list<string> $hashes
     * @return array<string, int>
     */
    public static function referencedBy(int $ownerId, array $hashes): array
    {
        $found = [];

        foreach (array_chunk(array_values(array_unique($hashes)), self::CHUNK) as $chunk) {
            $placeholders = implode(', ', array_fill(0, count($chunk), '?'));
            $statement = Database::pdo()->prepare(
                "SELECT b.sha256, b.size FROM blobs b
                 WHERE b.sha256 IN ($placeholders) AND b.sha256 IN (
                     SELECT vf.sha256 FROM version_files vf
                     JOIN instance_versions iv ON iv.id = vf.version_id
                     JOIN instances i ON i.id = iv.instance_id
                     WHERE i.owner_id = ?
                 )"
            );
            $statement->execute([...$chunk, $ownerId]);

            foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
                $found[(string) $row['sha256']] = (int) $row['size'];
            }
        }

        return $found;
    }

    /**
     * The sizes of the given blobs that the blobs table already holds. A hash missing from the result is not stored yet.
     *
     * @param list<string> $hashes
     * @return array<string, int>
     */
    public static function storedSizes(array $hashes): array
    {
        $found = [];

        foreach (array_chunk(array_values(array_unique($hashes)), self::CHUNK) as $chunk) {
            $placeholders = implode(', ', array_fill(0, count($chunk), '?'));
            $statement = Database::pdo()->prepare("SELECT sha256, size FROM blobs WHERE sha256 IN ($placeholders)");
            $statement->execute($chunk);

            foreach ($statement->fetchAll(PDO::FETCH_ASSOC) as $row) {
                $found[(string) $row['sha256']] = (int) $row['size'];
            }
        }

        return $found;
    }
}
