<?php

declare(strict_types=1);

namespace app\Config;

/**
 * Application configuration. Each key is read from the first source that defines it.
 *
 * In development (APP_ENV=development), api/.env and config/production.php come first, and the
 * process environment comes last. That stops variables set on a developer's machine from
 * redirecting the project. Everywhere else, real environment variables come first, then
 * config/production.php, then .env. An empty value counts as unset. set() is for tests and
 * beats every source.
 */
final class Config
{
    private static bool $loaded = false;
    private static bool $fileFirst = false;
    private static string $mode = 'production';
    private static ?string $apiDir = null;

    /** @var array<string, string> Values from config/production.php, then .env (production wins). */
    private static array $files = [];

    /** @var array<string, string> */
    private static array $overrides = [];

    public static function load(string $apiDir): void
    {
        if (self::$loaded) {
            return;
        }

        $values = [];

        $envFile = $apiDir . '/.env';
        if (is_file($envFile)) {
            $content = file_get_contents($envFile);
            if ($content === false) {
                throw new \RuntimeException("Cannot read $envFile");
            }
            $values = self::parseEnv($content);
        }

        $productionFile = $apiDir . '/config/production.php';
        if (is_file($productionFile)) {
            $production = require $productionFile;
            if (!is_array($production)) {
                throw new \RuntimeException('config/production.php must return an array');
            }
            foreach ($production as $key => $value) {
                if (is_string($key) && is_scalar($value)) {
                    $values[$key] = (string) $value;
                }
            }
        }

        self::$files = $values;
        self::$apiDir = $apiDir;
        self::$mode = self::resolveEnvironment($values);
        self::$fileFirst = self::$mode === 'development';
        self::$loaded = true;
    }

    public static function get(string $key, ?string $default = null): ?string
    {
        return self::lookup($key) ?? $default;
    }

    public static function need(string $key): string
    {
        $value = self::lookup($key);
        if ($value === null) {
            throw new \RuntimeException("Missing configuration: $key");
        }

        return $value;
    }

    /** Sets a value for the rest of the process. Tests only. */
    public static function set(string $key, string $value): void
    {
        self::$overrides[$key] = $value;
    }

    /** Clears every value and the loaded flag. Tests only. */
    public static function reset(): void
    {
        self::$loaded = false;
        self::$fileFirst = false;
        self::$mode = 'production';
        self::$apiDir = null;
        self::$files = [];
        self::$overrides = [];
    }

    public static function apiDir(): string
    {
        if (self::$apiDir === null) {
            throw new \RuntimeException('Configuration is not loaded');
        }

        return self::$apiDir;
    }

    public static function environment(): string
    {
        // An empty override counts as unset, like every other key.
        $override = self::$overrides['APP_ENV'] ?? '';
        if ($override !== '') {
            return $override;
        }

        return self::$mode;
    }

    public static function isProduction(): bool
    {
        return self::environment() === 'production';
    }

    private static function lookup(string $key): ?string
    {
        if (array_key_exists($key, self::$overrides)) {
            $value = self::$overrides[$key];

            return $value === '' ? null : $value;
        }

        if (self::$fileFirst) {
            return self::fileValue($key) ?? self::environmentValue($key);
        }

        return self::environmentValue($key) ?? self::fileValue($key);
    }

    private static function environmentValue(string $key): ?string
    {
        foreach ([getenv($key), $_ENV[$key] ?? null] as $candidate) {
            if (is_string($candidate) && $candidate !== '') {
                return $candidate;
            }
        }

        return null;
    }

    private static function fileValue(string $key): ?string
    {
        $value = self::$files[$key] ?? null;

        return ($value === null || $value === '') ? null : $value;
    }

    /**
     * APP_ENV as the process environment and the files give it, before the precedence rule applies.
     *
     * @param array<string, string> $values
     */
    private static function resolveEnvironment(array $values): string
    {
        foreach ([getenv('APP_ENV'), $_ENV['APP_ENV'] ?? null, $values['APP_ENV'] ?? null] as $candidate) {
            if (is_string($candidate) && $candidate !== '') {
                return $candidate;
            }
        }

        return 'production';
    }

    /**
     * @return array<string, string>
     */
    private static function parseEnv(string $content): array
    {
        $values = [];
        foreach (preg_split('/\r\n|\r|\n/', $content) ?: [] as $line) {
            $line = trim($line);
            if ($line === '' || $line[0] === '#') {
                continue;
            }
            if (str_starts_with($line, 'export ')) {
                $line = ltrim(substr($line, 7));
            }

            $equals = strpos($line, '=');
            if ($equals === false) {
                continue;
            }

            $key = trim(substr($line, 0, $equals));
            if (preg_match('/^[A-Za-z_][A-Za-z0-9_]*$/', $key) !== 1) {
                continue;
            }

            $values[$key] = self::parseValue(trim(substr($line, $equals + 1)));
        }

        return $values;
    }

    private static function parseValue(string $raw): string
    {
        $quote = $raw[0] ?? '';
        if ($quote === '"' || $quote === "'") {
            $close = strpos($raw, $quote, 1);
            if ($close !== false) {
                return substr($raw, 1, $close - 1);
            }
        }

        // Unquoted values lose a trailing " # comment". There is no variable expansion.
        return trim(preg_replace('/\s+#.*$/', '', $raw) ?? $raw);
    }
}
