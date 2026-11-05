<?php

declare(strict_types=1);

namespace tests\Storage;

use app\Auth\AuthException;
use app\Config\Config;
use app\Storage\BlobKeys;
use app\Storage\BlobStores;
use app\Storage\R2BlobStore;
use PHPUnit\Framework\TestCase;

/**
 * The R2 store's configuration checks and its presigned PUT, which must sign the checksum header. No request leaves the
 * process: the signing is computed locally.
 */
final class BlobStoresTest extends TestCase
{
    private const SHA = '2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e82c1cd1d1ad4f5a1c8a7f8c4a0';

    protected function tearDown(): void
    {
        BlobStores::use(null);
        Config::reset();

        parent::tearDown();
    }

    public function test_a_missing_or_empty_configuration_is_storage_unavailable(): void
    {
        BlobStores::use(null);
        Config::set('R2_ENDPOINT', '');
        Config::set('R2_ACCESS_KEY_ID', 'key');
        Config::set('R2_SECRET_ACCESS_KEY', 'secret');
        Config::set('R2_BUCKET', 'bucket');

        try {
            BlobStores::current();
            self::fail('current() returned without an endpoint.');
        } catch (AuthException $error) {
            self::assertSame('storage_unavailable', $error->errorCode);
            self::assertSame(503, $error->httpStatus);
        }
    }

    public function test_the_override_is_returned_and_removed_by_use_null(): void
    {
        $fake = new FakeBlobStore();
        BlobStores::use($fake);

        self::assertSame($fake, BlobStores::current());

        BlobStores::use(null);
        Config::set('R2_ENDPOINT', '');
        $this->expectException(AuthException::class);
        BlobStores::current();
    }

    public function test_the_r2_store_signs_the_checksum_header_into_a_presigned_put(): void
    {
        $this->configureR2();

        $store = BlobStores::current();
        self::assertInstanceOf(R2BlobStore::class, $store);

        $checksum = BlobKeys::base64Digest(self::SHA);
        $signed = $store->presignPut(BlobKeys::forSha256(self::SHA), 5, $checksum, 'upload-tag', 900);

        self::assertSame([
            'x-amz-checksum-sha256' => $checksum,
            'x-amz-meta-lodestone-upload' => 'upload-tag',
            'Content-Length' => '5',
        ], $signed['headers']);

        // The tag is signed, so a PUT cannot carry a tag other than the one its session was given.
        $url = urldecode($signed['url']);
        self::assertStringStartsWith('https://acct.r2.example.test/lodestone-test/blobs/' . self::SHA, $url);
        self::assertStringContainsString('X-Amz-Expires=900', $url);
        self::assertStringContainsString(
            'X-Amz-SignedHeaders=host;x-amz-checksum-sha256;x-amz-meta-lodestone-upload',
            $url,
        );
        self::assertStringContainsString('X-Amz-Signature=', $url);
    }

    public function test_the_r2_store_presigns_a_get_for_the_blob_key(): void
    {
        $this->configureR2();

        $url = BlobStores::current()->presignGet(BlobKeys::forSha256(self::SHA), 900, 'world.zip');

        self::assertStringStartsWith('https://acct.r2.example.test/lodestone-test/blobs/' . self::SHA, urldecode($url));
        self::assertStringContainsString('X-Amz-Expires=900', urldecode($url));
    }

    private function configureR2(): void
    {
        BlobStores::use(null);
        Config::set('R2_ENDPOINT', 'https://acct.r2.example.test');
        Config::set('R2_ACCESS_KEY_ID', 'test-access-key');
        Config::set('R2_SECRET_ACCESS_KEY', 'test-secret-key');
        Config::set('R2_BUCKET', 'lodestone-test');
    }
}
