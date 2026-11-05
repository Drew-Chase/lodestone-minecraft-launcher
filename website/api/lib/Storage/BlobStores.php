<?php

declare(strict_types=1);

namespace app\Storage;

use app\Auth\AuthException;
use app\Config\Config;

/**
 * Holds the blob store the sync routes use. Tests call use() with a fake, so the suite never reaches R2.
 */
final class BlobStores
{
    private static ?BlobStore $current = null;

    private function __construct()
    {
    }

    /**
     * The override set with use(), or else the R2 store built from R2_ENDPOINT, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY and
     * R2_BUCKET. An empty value counts as unset.
     *
     * @throws AuthException storage_unavailable (503) when the configuration is missing
     */
    public static function current(): BlobStore
    {
        if (self::$current !== null) {
            return self::$current;
        }

        $endpoint = Config::get('R2_ENDPOINT');
        $accessKeyId = Config::get('R2_ACCESS_KEY_ID');
        $secretAccessKey = Config::get('R2_SECRET_ACCESS_KEY');
        $bucket = Config::get('R2_BUCKET');

        if ($endpoint === null || $accessKeyId === null || $secretAccessKey === null || $bucket === null) {
            throw new AuthException('storage_unavailable', 503);
        }

        self::$current = new R2BlobStore(R2BlobStore::client($endpoint, $accessKeyId, $secretAccessKey), $bucket);

        return self::$current;
    }

    /** Replaces the store for the rest of the process. Null removes the override. Tests only. */
    public static function use(?BlobStore $store): void
    {
        self::$current = $store;
    }
}
