<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\RateLimiter;
use tests\Support\DatabaseTestCase;

final class RateLimiterTest extends DatabaseTestCase
{
    private const WINDOW = 900;

    private RateLimiter $limiter;

    protected function setUp(): void
    {
        parent::setUp();

        $this->limiter = new RateLimiter();
    }

    public function testNotBlockedBeforeAnyHit(): void
    {
        self::assertFalse($this->limiter->isBlocked('bucket:fresh', 3, self::WINDOW));
    }

    public function testBlockedOnceCountReachesLimit(): void
    {
        $bucket = 'bucket:reach';
        for ($i = 0; $i < 3; $i++) {
            $this->limiter->hit($bucket, self::WINDOW);
        }

        self::assertTrue($this->limiter->isBlocked($bucket, 3, self::WINDOW));
    }

    public function testNotBlockedOneBelowLimit(): void
    {
        $bucket = 'bucket:below';
        $this->limiter->hit($bucket, self::WINDOW);
        $this->limiter->hit($bucket, self::WINDOW);

        self::assertFalse($this->limiter->isBlocked($bucket, 3, self::WINDOW));
    }

    public function testHitReturnsTheRunningCount(): void
    {
        $bucket = 'bucket:count';

        self::assertSame(1, $this->limiter->hit($bucket, self::WINDOW));
        self::assertSame(2, $this->limiter->hit($bucket, self::WINDOW));
        self::assertSame(3, $this->limiter->hit($bucket, self::WINDOW));
    }

    public function testBucketsAreIndependent(): void
    {
        $this->limiter->hit('bucket:a', self::WINDOW);
        $this->limiter->hit('bucket:a', self::WINDOW);

        self::assertSame(1, $this->limiter->hit('bucket:b', self::WINDOW));
    }

    public function testClearResetsTheBucket(): void
    {
        $bucket = 'bucket:clear';
        for ($i = 0; $i < 3; $i++) {
            $this->limiter->hit($bucket, self::WINDOW);
        }
        self::assertTrue($this->limiter->isBlocked($bucket, 3, self::WINDOW));

        $this->limiter->clear($bucket);

        self::assertFalse($this->limiter->isBlocked($bucket, 3, self::WINDOW));
        self::assertSame(1, $this->limiter->hit($bucket, self::WINDOW));
    }

    public function testRetryAfterIsWithinTheWindow(): void
    {
        $retry = $this->limiter->retryAfter(self::WINDOW);

        self::assertGreaterThanOrEqual(1, $retry);
        self::assertLessThanOrEqual(self::WINDOW, $retry);
    }

    public function testBucketKeysAreNotStoredInPlainText(): void
    {
        $this->limiter->hit('login:email:secret@example.test', self::WINDOW);

        $count = $this->pdo()->query(
            "SELECT COUNT(*) FROM rate_limits WHERE bucket_key LIKE '%secret@example%'"
        )->fetchColumn();

        self::assertSame(0, (int) $count);
    }
}
