<?php

declare(strict_types=1);

namespace app\Storage;

/**
 * The object key and the checksum form of a blob. The key is derived from the SHA-256 hex digest, and the checksum is the
 * base64 of the raw 32 digest bytes, which is what S3-style stores compare.
 */
final class BlobKeys
{
    private const PREFIX = 'blobs/';

    private function __construct()
    {
    }

    /** @param string $sha256Hex 64 lowercase hex characters */
    public static function forSha256(string $sha256Hex): string
    {
        return self::PREFIX . $sha256Hex;
    }

    /** @param string $sha256Hex 64 lowercase hex characters */
    public static function base64Digest(string $sha256Hex): string
    {
        return base64_encode((string) hex2bin($sha256Hex));
    }
}
