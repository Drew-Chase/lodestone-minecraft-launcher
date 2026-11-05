<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * Holds the transport the providers use. Tests call use() with a fake, so the suite never reaches the network.
 */
final class Transports
{
    private static ?Transport $current = null;

    private function __construct()
    {
    }

    public static function current(): Transport
    {
        return self::$current ??= new CurlTransport();
    }

    public static function use(Transport $transport): void
    {
        self::$current = $transport;
    }
}
