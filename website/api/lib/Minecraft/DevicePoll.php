<?php

declare(strict_types=1);

namespace app\Minecraft;

/**
 * One answer from Microsoft's token endpoint while a device-code sign-in is waiting. The access token is set only
 * for AUTHORIZED, and it is used for the Xbox chain and then dropped.
 */
final readonly class DevicePoll
{
    public const PENDING = 'pending';
    public const SLOW_DOWN = 'slow_down';
    public const DECLINED = 'declined';
    public const EXPIRED = 'expired';
    public const AUTHORIZED = 'authorized';

    /** Microsoft answered that the sign-in can never succeed (bad_verification_code or invalid_grant). It cannot continue. */
    public const FAILED = 'failed';

    public function __construct(
        public string $state,
        public ?string $accessToken = null,
    ) {
    }
}
