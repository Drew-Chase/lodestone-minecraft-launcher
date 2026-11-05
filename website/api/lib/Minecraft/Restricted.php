<?php

declare(strict_types=1);

namespace app\Minecraft;

/**
 * The Microsoft account can have Minecraft, but Xbox Live does not allow it here: the country or region does not
 * support Xbox Live, adult verification is required first, or a minor is outside a family. Not a service failure,
 * so it is not retried.
 */
final class Restricted extends \RuntimeException
{
}
