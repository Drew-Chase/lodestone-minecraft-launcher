<?php

declare(strict_types=1);

namespace app\Auth;

/** Never reports a breach. Used by tests. */
final class NullBreachChecker implements BreachChecker
{
    public function isBreached(string $password): bool
    {
        return false;
    }
}
