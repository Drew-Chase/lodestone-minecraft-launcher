<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\HibpBreachChecker;
use PHPUnit\Framework\TestCase;

final class HibpBreachCheckerTest extends TestCase
{
    private const SUFFIX = '1E4C9B93F3F0682250B6CF8331B7EE68FD8';
    private const OTHER = '0000000000000000000000000000000000A';

    public function testMatchesReturnsTrueForASuffixWithACount(): void
    {
        $body = self::OTHER . ":4\n" . self::SUFFIX . ":42\n";

        self::assertTrue(HibpBreachChecker::matches($body, self::SUFFIX));
    }

    public function testMatchesReturnsFalseWhenTheCountIsZero(): void
    {
        // Padding lines are sent with a count of 0.
        $body = self::SUFFIX . ":0\n" . self::OTHER . ":3\n";

        self::assertFalse(HibpBreachChecker::matches($body, self::SUFFIX));
    }

    public function testMatchesReturnsFalseForADifferentSuffix(): void
    {
        $body = self::OTHER . ":5\n";

        self::assertFalse(HibpBreachChecker::matches($body, self::SUFFIX));
    }

    public function testMatchesRequiresTheWholeSuffix(): void
    {
        // The suffix is only a prefix of this line, so it is a different password.
        $body = self::SUFFIX . "X:5\n";

        self::assertFalse(HibpBreachChecker::matches($body, self::SUFFIX));
    }

    public function testMatchesAcceptsCrlfLineEndings(): void
    {
        $body = self::OTHER . ":1\r\n" . self::SUFFIX . ":7\r\n";

        self::assertTrue(HibpBreachChecker::matches($body, self::SUFFIX));
    }

    public function testMatchesIgnoresTheCaseOfTheSuffix(): void
    {
        $body = self::OTHER . ":1\r\n" . self::SUFFIX . ":7\r\n";

        self::assertTrue(HibpBreachChecker::matches($body, strtolower(self::SUFFIX)));
        self::assertTrue(HibpBreachChecker::matches(strtolower($body), self::SUFFIX));
    }

    public function testMatchesOnAnEmptyBodyIsFalse(): void
    {
        self::assertFalse(HibpBreachChecker::matches('', self::SUFFIX));
    }

    public function testIsBreachedFailsOpenWhenTheEndpointIsUnreachable(): void
    {
        // Port 9 is closed here, so the request fails at once and never reaches the real service.
        $checker = new HibpBreachChecker('http://127.0.0.1:9/');

        self::assertFalse($checker->isBreached('correct horse battery'));
    }
}
