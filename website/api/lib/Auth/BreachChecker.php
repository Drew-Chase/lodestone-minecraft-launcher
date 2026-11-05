<?php

declare(strict_types=1);

namespace app\Auth;

interface BreachChecker
{
    /** True when the password appears in a breach list. */
    public function isBreached(string $password): bool;
}
