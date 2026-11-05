<?php

declare(strict_types=1);

namespace app\Storage;

/**
 * Object storage for the content-addressed blobs. A key names the SHA-256 of the object's bytes, so an object never changes
 * once it is stored. Tests replace the implementation with a fake through BlobStores::use().
 */
interface BlobStore
{
    /**
     * A presigned PUT for one object. The client must send every header in the result with the upload, or the signature fails.
     * The tag names the upload session that asked for the PUT. The store keeps it with the object, so head() reports which
     * session last wrote the object.
     *
     * @return array{url: string, headers: array<string, string>}
     */
    public function presignPut(string $key, int $size, string $sha256Base64, string $tag, int $ttlSeconds): array;

    /** A presigned GET for one object. The file name is the name a download is saved under. */
    public function presignGet(string $key, int $ttlSeconds, string $fileName): string;

    /**
     * The stored object's size, checksum and upload tag, or null when no object has the key. The checksum is the base64
     * SHA-256 the store reports, or null when it reports none. The tag is null when the last write carried none.
     *
     * @return array{size: int, sha256: ?string, tag: ?string}|null
     */
    public function head(string $key): ?array;
}
