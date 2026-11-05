<?php

declare(strict_types=1);

namespace app\Devices;

/**
 * Eight-character codes that people type or read aloud. The alphabet leaves out 0, 1, I, L, O and U.
 * Device user codes and friend codes both use it.
 */
final class ShortCode
{
    public const ALPHABET = 'ABCDEFGHJKMNPQRSTVWXYZ23456789';
    public const LENGTH = 8;

    private function __construct()
    {
    }

    public static function random(): string
    {
        $max = strlen(self::ALPHABET) - 1;
        $code = '';
        for ($index = 0; $index < self::LENGTH; $index++) {
            $code .= self::ALPHABET[random_int(0, $max)];
        }

        return $code;
    }

    /**
     * Accepts spaces and hyphens anywhere, and any case. Returns the stored form (uppercase, no separators), or null
     * when what is left is not a code.
     */
    public static function normalize(string $input): ?string
    {
        $code = strtoupper((string) preg_replace('/[\s-]+/', '', $input));

        return preg_match('/^[' . self::ALPHABET . ']{' . self::LENGTH . '}$/D', $code) === 1 ? $code : null;
    }

    /** XXXX-XXXX, the form shown to people. */
    public static function hyphenate(string $code): string
    {
        return substr($code, 0, 4) . '-' . substr($code, 4);
    }
}
