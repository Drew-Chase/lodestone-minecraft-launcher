<?php

declare(strict_types=1);

namespace tests\Sync;

use app\Auth\AuthException;
use app\Sync\Quota;
use tests\Support\DatabaseTestCase;

/** Used bytes and the refusal past the allowance. Storage is counted once per distinct blob, against the owner. */
final class QuotaTest extends DatabaseTestCase
{
    use SyncFixtures;

    private const TWO_GIB = 2147483648;

    public function test_used_bytes_count_each_distinct_blob_once_across_the_owners_instances(): void
    {
        $owner = $this->member('owner@example.test', 'Owner', true, 'solo');
        $first = $this->instance($owner, 'One');
        $second = $this->instance($owner, 'Two');

        $a = ['sha256' => $this->digest('a'), 'size' => 10];
        $b = ['sha256' => $this->digest('b'), 'size' => 20];

        $this->insertVersion($first, $owner, [
            ['path' => 'a.txt', ...$a],
            ['path' => 'copy/a.txt', ...$a],
            ['path' => 'b.txt', ...$b],
        ]);
        $this->insertVersion($second, $owner, [['path' => 'a.txt', ...$a]]);

        self::assertSame(30, Quota::usedBytes($owner));
    }

    public function test_only_blobs_referenced_by_a_version_count(): void
    {
        $owner = $this->member('owner@example.test', 'Owner', true, 'solo');
        $this->instance($owner);
        $this->blob($this->digest('orphan'), 999);

        self::assertSame(0, Quota::usedBytes($owner));
    }

    public function test_a_share_does_not_count_against_the_editor(): void
    {
        $owner = $this->member('owner@example.test', 'Owner', true, 'solo');
        $editor = $this->member('editor@example.test', 'Editor', true, 'solo');
        $instance = $this->instance($owner);
        $this->share($instance, $editor, 'editor');

        $this->insertVersion($instance, $owner, [['path' => 'a.txt', 'sha256' => $this->digest('a'), 'size' => 50]]);

        self::assertSame(50, Quota::usedBytes($owner));
        self::assertSame(0, Quota::usedBytes($editor));
    }

    public function test_a_new_blob_past_the_allowance_is_refused(): void
    {
        $owner = $this->member('owner@example.test', 'Owner', true, 'solo');
        $instance = $this->instance($owner);

        // 46 blobs of 2 GiB is about 98.8 GB, inside the 100 GB Solo allowance.
        $files = [];
        for ($i = 0; $i < 46; $i++) {
            $files[] = ['path' => "world/$i.bin", 'sha256' => $this->digest("blob-$i"), 'size' => self::TWO_GIB];
        }
        $this->insertVersion($instance, $owner, $files);

        Quota::assertRoom($owner, 1073741824);

        $this->assertQuotaExceeded(fn () => Quota::assertRoom($owner, self::TWO_GIB));
    }

    public function test_a_free_plan_has_no_storage_to_use(): void
    {
        $owner = $this->member('free@example.test', 'Free', true, null);

        self::assertSame(0, Quota::limitBytes($owner));
        $this->assertQuotaExceeded(fn () => Quota::assertRoom($owner, 1));
    }

    private function assertQuotaExceeded(callable $check): void
    {
        try {
            $check();
        } catch (AuthException $error) {
            self::assertSame('quota_exceeded', $error->errorCode);
            self::assertSame(409, $error->httpStatus);

            return;
        }

        self::fail('The write was allowed past the allowance.');
    }
}
