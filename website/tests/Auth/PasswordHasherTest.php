<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\PasswordHasher;
use PHPUnit\Framework\TestCase;

final class PasswordHasherTest extends TestCase
{
    public function testVerifyAcceptsTheRightPassword(): void
    {
        $hasher = new PasswordHasher();
        $hash = $hasher->hash('correct horse battery');

        self::assertTrue($hasher->verify('correct horse battery', $hash));
    }

    public function testVerifyRejectsAWrongPassword(): void
    {
        $hasher = new PasswordHasher();
        $hash = $hasher->hash('correct horse battery');

        self::assertFalse($hasher->verify('wrong horse battery', $hash));
    }

    public function testHashIsNotThePlainPassword(): void
    {
        $hash = (new PasswordHasher())->hash('correct horse battery');

        self::assertStringNotContainsString('correct horse battery', $hash);
    }

    public function testHashThenVerifyWorksForALongPasswordWithTheDefaultAlgorithm(): void
    {
        $hasher = new PasswordHasher();
        $password = str_repeat('x', 200);
        $hash = $hasher->hash($password);

        self::assertTrue($hasher->verify($password, $hash));
        self::assertFalse($hasher->verify(str_repeat('x', 199), $hash));
        self::assertFalse($hasher->needsRehash($hash));
    }

    public function testFreshHashDoesNotNeedRehash(): void
    {
        $hasher = new PasswordHasher();

        self::assertFalse($hasher->needsRehash($hasher->hash('correct horse battery')));
    }

    public function testBcryptCountsTheWholePassword(): void
    {
        $hasher = new PasswordHasher(PASSWORD_BCRYPT);
        $shared = str_repeat('a', 100);
        $first = $shared . str_repeat('b', 100);
        $second = $shared . str_repeat('c', 100);

        $hash = $hasher->hash($first);

        // Both passwords agree on their first 100 bytes. Plain bcrypt would read only 72 of them and accept both.
        self::assertTrue($hasher->verify($first, $hash));
        self::assertFalse($hasher->verify($second, $hash));
    }

    public function testArgon2idHashVerifiesWithTheSameClass(): void
    {
        if (!defined('PASSWORD_ARGON2ID')) {
            self::markTestSkipped('This PHP build has no argon2id.');
        }

        $hasher = new PasswordHasher(PASSWORD_ARGON2ID);
        $hash = $hasher->hash('correct horse battery');

        self::assertStringStartsWith('$argon2id$', $hash);
        self::assertTrue($hasher->verify('correct horse battery', $hash));
        self::assertFalse($hasher->verify('wrong horse battery', $hash));
        self::assertFalse($hasher->needsRehash($hash));
    }

    public function testBcryptHashNeedsRehashOnceArgon2idIsAvailable(): void
    {
        if (!defined('PASSWORD_ARGON2ID')) {
            self::markTestSkipped('This PHP build has no argon2id.');
        }

        $bcrypt = (new PasswordHasher(PASSWORD_BCRYPT))->hash('correct horse battery');

        self::assertTrue((new PasswordHasher(PASSWORD_ARGON2ID))->needsRehash($bcrypt));
    }

    public function testBurnTimeDoesNotThrow(): void
    {
        $hasher = new PasswordHasher();
        $hasher->burnTime();
        $hasher->burnTime();

        $this->addToAssertionCount(1);
    }
}
