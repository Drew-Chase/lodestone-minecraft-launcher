<?php

declare(strict_types=1);

namespace app\Minecraft;

use DateTimeImmutable;
use DateTimeZone;

final class Times
{
    private function __construct()
    {
    }

    /** Formats a UTC database time as ISO 8601. */
    public static function iso(string $databaseTime): string
    {
        return (new DateTimeImmutable($databaseTime, new DateTimeZone('UTC')))->format(DATE_ATOM);
    }
}
