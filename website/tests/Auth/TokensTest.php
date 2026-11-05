<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\Tokens;
use PHPUnit\Framework\TestCase;

final class TokensTest extends TestCase
{
    public function testRandomIsSixtyFourLowercaseHexCharacters(): void
    {
        self::assertMatchesRegularExpression('/^[a-f0-9]{64}$/', Tokens::random());
    }

    public function testTwoRandomTokensDiffer(): void
    {
        self::assertNotSame(Tokens::random(), Tokens::random());
    }

    public function testHashIsSha256HexOfItsInput(): void
    {
        self::assertSame(hash('sha256', 'abc'), Tokens::hash('abc'));
    }

    public function testHashIsDeterministic(): void
    {
        $token = Tokens::random();

        self::assertSame(Tokens::hash($token), Tokens::hash($token));
        self::assertSame(64, strlen(Tokens::hash($token)));
    }

    public function testIsWellFormedAcceptsARandomToken(): void
    {
        self::assertTrue(Tokens::isWellFormed(Tokens::random()));
    }

    public function testIsWellFormedRejectsUppercase(): void
    {
        self::assertFalse(Tokens::isWellFormed(strtoupper(Tokens::random())));
    }

    public function testIsWellFormedRejectsShortTokens(): void
    {
        self::assertFalse(Tokens::isWellFormed(substr(Tokens::random(), 1)));
    }

    public function testIsWellFormedRejectsNonHexCharacters(): void
    {
        self::assertFalse(Tokens::isWellFormed(str_repeat('g', 64)));
    }

    public function testIsWellFormedRejectsATrailingNewline(): void
    {
        // The D modifier stops $ from matching before a final newline.
        self::assertFalse(Tokens::isWellFormed(Tokens::random() . "\n"));
    }

    public function testFamilyIdIsThirtyTwoLowercaseHexCharacters(): void
    {
        self::assertMatchesRegularExpression('/^[a-f0-9]{32}$/', Tokens::familyId());
    }
}
