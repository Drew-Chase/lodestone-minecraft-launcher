<?php

declare(strict_types=1);

namespace app\Minecraft;

/**
 * Microsoft's answer to a device-code request. The device code stays on the server; only the user code and the
 * verification address are shown to the person.
 */
final readonly class DeviceCode
{
    public function __construct(
        public string $deviceCode,
        public string $userCode,
        public string $verificationUri,
        public int $expiresIn,
        public int $interval,
    ) {
    }
}
