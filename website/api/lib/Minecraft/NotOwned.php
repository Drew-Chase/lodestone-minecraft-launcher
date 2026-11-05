<?php

declare(strict_types=1);

namespace app\Minecraft;

/**
 * The Microsoft account cannot play Minecraft Java Edition: it has no Xbox account, or it has no Minecraft profile.
 * A minor outside a family is Restricted instead. Not a service failure, so it is not retried.
 */
final class NotOwned extends \RuntimeException
{
}
