<?php

declare(strict_types=1);

namespace app\Sync;

use app\Auth\AuthException;

/**
 * Checks an upload manifest, the list of files an instance version holds. Every failure is invalid_manifest (400),
 * whichever rule it broke.
 */
final class Manifest
{
    public const MAX_FILES = 50000;
    public const MAX_PATH_BYTES = 512;
    public const MAX_SIZE = 2147483648;

    /**
     * The largest manifest body. Each entry is at most about 700 bytes of JSON (a 512-byte path, a digest and two numbers),
     * so 50,000 entries fit in roughly 35 MB; the cap leaves headroom above that.
     */
    public const MAX_BODY_BYTES = 67108864;

    private const SHA256_PATTERN = '/^[0-9a-f]{64}$/D';

    private function __construct()
    {
    }

    /**
     * Returns the files in the form they are stored: path, sha256, size and mtime_ms, in the order given.
     *
     * @return list<array{path: string, sha256: string, size: int, mtime_ms: int}>
     * @throws AuthException invalid_manifest (400)
     */
    public static function validate(mixed $files): array
    {
        if (!is_array($files) || !array_is_list($files) || $files === [] || count($files) > self::MAX_FILES) {
            throw self::invalid();
        }

        $paths = [];
        $sizes = [];
        $checked = [];

        foreach ($files as $entry) {
            $file = self::entry($entry);

            if (isset($paths[$file['path']])) {
                throw self::invalid();
            }
            $paths[$file['path']] = true;

            // The same bytes cannot have two sizes.
            if (isset($sizes[$file['sha256']]) && $sizes[$file['sha256']] !== $file['size']) {
                throw self::invalid();
            }
            $sizes[$file['sha256']] = $file['size'];

            $checked[] = $file;
        }

        return $checked;
    }

    /**
     * Each distinct blob the files name, with its size, keyed by SHA-256. Call it on a validated manifest.
     *
     * @param list<array{sha256: string, size: int}> $files
     * @return array<string, int>
     */
    public static function blobs(array $files): array
    {
        $blobs = [];
        foreach ($files as $file) {
            $blobs[$file['sha256']] = $file['size'];
        }

        return $blobs;
    }

    /** The sum of every file's size. Duplicate content counts once per file. */
    public static function totalBytes(array $files): int
    {
        return (int) array_sum(array_column($files, 'size'));
    }

    /**
     * A relative path with forward slashes: 1 to 512 bytes of valid UTF-8, no control characters, no leading slash,
     * no backslash, no drive letter, and no empty, "." or ".." segment.
     */
    public static function validPath(string $path): bool
    {
        $bytes = strlen($path);
        if ($bytes < 1 || $bytes > self::MAX_PATH_BYTES) {
            return false;
        }

        if (preg_match('//u', $path) !== 1 || preg_match('/\p{Cc}/u', $path) === 1) {
            return false;
        }

        if (str_starts_with($path, '/') || str_contains($path, '\\') || preg_match('/^[A-Za-z]:/', $path) === 1) {
            return false;
        }

        foreach (explode('/', $path) as $segment) {
            if ($segment === '' || $segment === '.' || $segment === '..') {
                return false;
            }
        }

        return true;
    }

    /** @return array{path: string, sha256: string, size: int, mtime_ms: int} */
    private static function entry(mixed $entry): array
    {
        // Decoded JSON objects arrive as stdClass; arrays with string keys are accepted as well.
        if (is_object($entry)) {
            $entry = get_object_vars($entry);
        }

        if (!is_array($entry) || array_is_list($entry)) {
            throw self::invalid();
        }

        $path = $entry['path'] ?? null;
        $sha256 = $entry['sha256'] ?? null;
        $size = $entry['size'] ?? null;
        $mtime = $entry['mtime_ms'] ?? null;

        if (!is_string($path) || !self::validPath($path)) {
            throw self::invalid();
        }

        if (!is_string($sha256) || preg_match(self::SHA256_PATTERN, $sha256) !== 1) {
            throw self::invalid();
        }

        if (!is_int($size) || $size < 0 || $size > self::MAX_SIZE) {
            throw self::invalid();
        }

        if (!is_int($mtime) || $mtime < 0) {
            throw self::invalid();
        }

        return ['path' => $path, 'sha256' => $sha256, 'size' => $size, 'mtime_ms' => $mtime];
    }

    private static function invalid(): AuthException
    {
        return new AuthException('invalid_manifest', 400);
    }
}
