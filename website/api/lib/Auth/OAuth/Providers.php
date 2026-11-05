<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use app\Config\Config;

/**
 * Which providers are configured, and how to build one. A provider is enabled only when its client ID and secret are both set.
 */
final class Providers
{
    private const NAMES = ['google', 'microsoft', 'github'];

    private function __construct()
    {
    }

    /** @return list<string> */
    public static function enabled(): array
    {
        $enabled = [];
        foreach (self::NAMES as $name) {
            $prefix = strtoupper($name);
            if (Config::get($prefix . '_CLIENT_ID') !== null && Config::get($prefix . '_CLIENT_SECRET') !== null) {
                $enabled[] = $name;
            }
        }

        return $enabled;
    }

    public static function make(string $name): ?Provider
    {
        return match ($name) {
            'google' => new GoogleProvider(),
            'microsoft' => new MicrosoftProvider(),
            'github' => new GitHubProvider(),
            default => null,
        };
    }
}
