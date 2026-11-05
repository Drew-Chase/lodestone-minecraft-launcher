<?php

declare(strict_types=1);

namespace app\Devices;

use DateTimeImmutable;
use DateTimeZone;

/**
 * Stored DATETIME(6) values are UTC. This turns one into the ISO 8601 text the API returns.
 */
final class Times
{
    private function __construct()
    {
    }

    public static function iso(?string $stored): ?string
    {
        if ($stored === null) {
            return null;
        }

        return (new DateTimeImmutable($stored, new DateTimeZone('UTC')))->format(DATE_ATOM);
    }
}
