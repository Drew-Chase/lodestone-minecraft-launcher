<?php

declare(strict_types=1);

namespace app\Auth;

/**
 * The password rules from the README, checked in order: length in characters, size in bytes, then breach lists.
 */
final class PasswordPolicy
{
    private const MIN_CHARACTERS = 12;
    private const MAX_BYTES = 256;

    public function __construct(private readonly BreachChecker $breaches)
    {
    }

    /**
     * @throws AuthException password_too_short, password_too_long or password_breached (all 400)
     */
    public function check(string $password): void
    {
        if (mb_strlen($password, 'UTF-8') < self::MIN_CHARACTERS) {
            throw new AuthException('password_too_short', 400);
        }

        if (strlen($password) > self::MAX_BYTES) {
            throw new AuthException('password_too_long', 400);
        }

        if ($this->breaches->isBreached($password)) {
            throw new AuthException('password_breached', 400);
        }
    }
}
