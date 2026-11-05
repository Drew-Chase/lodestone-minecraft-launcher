<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\AuthException;
use app\Auth\BreachChecker;
use app\Auth\NullBreachChecker;
use app\Auth\PasswordPolicy;
use PHPUnit\Framework\TestCase;

final class PasswordPolicyTest extends TestCase
{
    public function testElevenCharactersIsTooShort(): void
    {
        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('password_too_short');

        $this->policy()->check(str_repeat('a', 11));
    }

    public function testTwelveCharactersPasses(): void
    {
        $this->policy()->check(str_repeat('a', 12));

        $this->addToAssertionCount(1);
    }

    public function testTwelveMultibyteCharactersPassEvenWithMoreBytes(): void
    {
        // 12 characters, 24 bytes: the length rule counts characters.
        $this->policy()->check(str_repeat('é', 12));

        $this->addToAssertionCount(1);
    }

    public function testElevenMultibyteCharactersIsTooShort(): void
    {
        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('password_too_short');

        $this->policy()->check(str_repeat('é', 11));
    }

    public function testTwoHundredFiftySevenBytesIsTooLong(): void
    {
        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('password_too_long');

        $this->policy()->check(str_repeat('a', 257));
    }

    public function testTwoHundredFiftySixBytesPasses(): void
    {
        $this->policy()->check(str_repeat('a', 256));

        $this->addToAssertionCount(1);
    }

    public function testBreachedPasswordIsRejected(): void
    {
        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('password_breached');

        $this->policy(new class implements BreachChecker {
            public function isBreached(string $password): bool
            {
                return true;
            }
        })->check('correct horse battery staple');
    }

    public function testShortBreachedPasswordReportsTooShortFirst(): void
    {
        $breached = new class implements BreachChecker {
            public function isBreached(string $password): bool
            {
                return true;
            }
        };

        try {
            $this->policy($breached)->check('short');
            self::fail('A short password must be rejected.');
        } catch (AuthException $error) {
            self::assertSame('password_too_short', $error->errorCode);
            self::assertSame(400, $error->httpStatus);
        }
    }

    private function policy(?BreachChecker $breaches = null): PasswordPolicy
    {
        return new PasswordPolicy($breaches ?? new NullBreachChecker());
    }
}
