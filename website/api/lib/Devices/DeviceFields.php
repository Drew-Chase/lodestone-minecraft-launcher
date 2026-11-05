<?php

declare(strict_types=1);

namespace app\Devices;

use app\Auth\AuthException;

/**
 * The name, operating system and version a launcher sends when it starts a device sign-in.
 */
final class DeviceFields
{
    private const LIMITS = ['name' => 64, 'os' => 32, 'app_version' => 32];

    private function __construct()
    {
    }

    /**
     * @param array<string, mixed> $body
     * @return array{name: string, os: string, app_version: string}
     * @throws AuthException missing_field (400) when a field is absent or not a string, invalid_device (400) when out of range
     */
    public static function read(array $body): array
    {
        foreach (array_keys(self::LIMITS) as $key) {
            if (!is_string($body[$key] ?? null)) {
                throw new AuthException('missing_field', 400);
            }
        }

        $fields = [];
        foreach (self::LIMITS as $key => $max) {
            $fields[$key] = self::text((string) $body[$key], $max);
        }

        return $fields;
    }

    private static function text(string $value, int $max): string
    {
        $trimmed = trim($value);
        $length = mb_strlen($trimmed, 'UTF-8');

        if ($length < 1 || $length > $max || preg_match('/[\x00-\x1F\x7F]/u', $trimmed) !== 0) {
            throw new AuthException('invalid_device', 400);
        }

        return $trimmed;
    }
}
