<?php

declare(strict_types=1);

namespace app\Storage;

use app\Auth\AuthException;
use Aws\Exception\AwsException;
use Aws\S3\S3Client;

/**
 * Blobs in Cloudflare R2 through its S3-compatible API. The SDK signs every presigned URL with the account's secret key, so
 * clients never see the key.
 */
final class R2BlobStore implements BlobStore
{
    private const REGION = 'auto';

    /** The metadata key that holds the upload tag. S3 returns it lower-cased. */
    private const TAG_KEY = 'lodestone-upload';

    public function __construct(
        private readonly S3Client $client,
        private readonly string $bucket,
    ) {
    }

    /**
     * An S3 client for the account's R2 endpoint. Path-style, because R2 serves the bucket under the endpoint host.
     * Checksums are added only where a request names one, so a presigned PUT carries exactly the checksum the caller sends.
     */
    public static function client(string $endpoint, string $accessKeyId, string $secretAccessKey): S3Client
    {
        return new S3Client([
            'version' => 'latest',
            'region' => self::REGION,
            'endpoint' => $endpoint,
            'use_path_style_endpoint' => true,
            'credentials' => ['key' => $accessKeyId, 'secret' => $secretAccessKey],
            'request_checksum_calculation' => 'when_required',
        ]);
    }

    /**
     * The SDK moves the checksum header into the query, and also keeps it among the signed headers. The client sends it
     * as a header, so the signature covers it. The upload tag travels the same way, as object metadata.
     */
    public function presignPut(string $key, int $size, string $sha256Base64, string $tag, int $ttlSeconds): array
    {
        $command = $this->client->getCommand('PutObject', [
            'Bucket' => $this->bucket,
            'Key' => $key,
            'ContentLength' => $size,
            'ChecksumSHA256' => $sha256Base64,
            'Metadata' => [self::TAG_KEY => $tag],
        ]);

        $request = $this->client->createPresignedRequest($command, $this->expiry($ttlSeconds));

        return [
            'url' => (string) $request->getUri(),
            'headers' => [
                'x-amz-checksum-sha256' => $sha256Base64,
                'x-amz-meta-' . self::TAG_KEY => $tag,
                'Content-Length' => (string) $size,
            ],
        ];
    }

    public function presignGet(string $key, int $ttlSeconds, string $fileName): string
    {
        // Downloads save under the file's own name. Browsers ignore the download attribute on cross-origin links,
        // so the name is set by the storage response. Inline previews are unaffected by the attachment header.
        $ascii = preg_replace('/[^A-Za-z0-9._-]/', '_', $fileName) ?? 'file';
        $disposition = 'attachment; filename="' . $ascii . '"; filename*=UTF-8\'\'' . rawurlencode($fileName);

        $command = $this->client->getCommand('GetObject', [
            'Bucket' => $this->bucket,
            'Key' => $key,
            'ResponseContentDisposition' => $disposition,
        ]);

        return (string) $this->client->createPresignedRequest($command, $this->expiry($ttlSeconds))->getUri();
    }

    public function head(string $key): ?array
    {
        try {
            $result = $this->client->headObject([
                'Bucket' => $this->bucket,
                'Key' => $key,
                'ChecksumMode' => 'ENABLED',
            ]);
        } catch (AwsException $error) {
            if ($error->getStatusCode() === 404 || $error->getAwsErrorCode() === 'NotFound') {
                return null;
            }

            throw new AuthException('storage_unavailable', 503);
        }

        $checksum = $result['ChecksumSHA256'] ?? null;
        $tag = $result['Metadata'][self::TAG_KEY] ?? null;

        return [
            'size' => (int) $result['ContentLength'],
            'sha256' => is_string($checksum) ? $checksum : null,
            'tag' => is_string($tag) ? $tag : null,
        ];
    }

    private function expiry(int $ttlSeconds): string
    {
        return '+' . $ttlSeconds . ' seconds';
    }
}
