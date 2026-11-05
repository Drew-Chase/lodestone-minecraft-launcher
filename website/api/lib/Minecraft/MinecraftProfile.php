<?php

declare(strict_types=1);

namespace app\Minecraft;

/**
 * A Minecraft profile as the account service reports it: the dashed, lower-case uuid and the current name.
 */
final readonly class MinecraftProfile
{
    public function __construct(
        public string $uuid,
        public string $name,
    ) {
    }
}
