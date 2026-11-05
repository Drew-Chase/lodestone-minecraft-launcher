<?php

declare(strict_types=1);

namespace app\Http;

use Psr\Http\Message\ServerRequestInterface;

/**
 * The connection's address. Forwarded headers are never read, since clients can set them.
 */
final class ClientIp
{
    private function __construct()
    {
    }

    public static function from(ServerRequestInterface $request): string
    {
        $address = $request->getServerParams()['REMOTE_ADDR'] ?? null;

        return is_string($address) && $address !== '' ? $address : '0.0.0.0';
    }
}
