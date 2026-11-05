<?php

declare(strict_types=1);

namespace app\Sync;

use DateTimeImmutable;
use DateTimeZone;

/** Stored DATETIME(6) values are UTC. This turns one into the ISO 8601 text the API returns. */
final class Times
{
    private function __construct()
    {
    }

    public static function iso(string $databaseTime): string
    {
        return (new DateTimeImmutable($databaseTime, new DateTimeZone('UTC')))->format(DATE_ATOM);
    }
}
