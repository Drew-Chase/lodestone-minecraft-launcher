<?php

declare(strict_types=1);

namespace tests\Storage;

use app\Storage\BlobKeys;
use app\Storage\BlobStore;
use LogicException;

/**
 * An in-memory blob store. A test marks objects as stored with markStored() or put(), and every call is recorded so a test
 * can see what the routes asked for. The presigned URLs are fake, but they carry the key and the TTL.
 */
final class FakeBlobStore implements BlobStore
{
    /** @var array<string, array{size: int, sha256: ?string, tag: ?string}> Stored objects by key. */
    private array $objects = [];

    /** @var array<string, string> The tag of the latest presigned PUT for each key, which put() writes with. */
    private array $presigned = [];

    /** @var list<array{method: string, key: string, ttl: int}> */
    private array $calls = [];

    /**
     * Marks the object for a blob as stored. The checksum defaults to the correct base64 digest of the hex SHA-256; pass
     * another value, or null, to simulate a mismatch. The tag is the upload session that wrote the object, or null for none.
     */
    public function markStored(string $sha256Hex, int $size, ?string $checksumBase64 = null, ?string $tag = null): void
    {
        $this->objects[BlobKeys::forSha256($sha256Hex)] = [
            'size' => $size,
            'sha256' => $checksumBase64 ?? BlobKeys::base64Digest($sha256Hex),
            'tag' => $tag,
        ];
    }

    /**
     * Stores the object the way a client's PUT does: the tag is the session of the presigned URL the client was given.
     * Fails when no PUT was presigned for the blob, since a client cannot write one it was never given.
     */
    public function put(string $sha256Hex, int $size, ?string $checksumBase64 = null): void
    {
        $key = BlobKeys::forSha256($sha256Hex);
        if (!isset($this->presigned[$key])) {
            throw new LogicException('No PUT was presigned for ' . $key . '.');
        }

        $this->markStored($sha256Hex, $size, $checksumBase64, $this->presigned[$key]);
    }

    /** @return list<array{method: string, key: string, ttl: int}> */
    public function calls(): array
    {
        return $this->calls;
    }

    public function presignPut(string $key, int $size, string $sha256Base64, string $tag, int $ttlSeconds): array
    {
        $this->calls[] = ['method' => 'PUT', 'key' => $key, 'ttl' => $ttlSeconds];
        $this->presigned[$key] = $tag;

        return [
            'url' => 'https://blobs.test/put/' . $key . '?ttl=' . $ttlSeconds,
            'headers' => [
                'x-amz-checksum-sha256' => $sha256Base64,
                'x-amz-meta-lodestone-upload' => $tag,
                'Content-Length' => (string) $size,
            ],
        ];
    }

    public function presignGet(string $key, int $ttlSeconds, string $fileName): string
    {
        $this->calls[] = ['method' => 'GET', 'key' => $key, 'ttl' => $ttlSeconds, 'name' => $fileName];

        return 'https://blobs.test/get/' . $key . '?ttl=' . $ttlSeconds;
    }

    public function head(string $key): ?array
    {
        $this->calls[] = ['method' => 'HEAD', 'key' => $key, 'ttl' => 0];

        return $this->objects[$key] ?? null;
    }
}
