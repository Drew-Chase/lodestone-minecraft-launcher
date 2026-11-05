<?php

declare(strict_types=1);

namespace app\Minecraft;

final class Uuid
{
    private const PATTERN = '/^[0-9a-f]{8}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{12}$/iD';

    private function __construct()
    {
    }

    /** The lower-case dashed form of a uuid written with or without dashes, or null when it is not one. */
    public static function dashed(string $value): ?string
    {
        if (preg_match(self::PATTERN, $value) !== 1) {
            return null;
        }

        $hex = strtolower(str_replace('-', '', $value));

        return substr($hex, 0, 8) . '-' . substr($hex, 8, 4) . '-' . substr($hex, 12, 4)
            . '-' . substr($hex, 16, 4) . '-' . substr($hex, 20, 12);
    }
}
