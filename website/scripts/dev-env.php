<?php

declare(strict_types=1);

// Creates website/api/.env from website/api/.env.example and generates JWT_SECRET.
// It never overwrites an existing .env and never prints a secret value.

$apiDir = dirname(__DIR__) . '/api';
$envPath = $apiDir . '/.env';
$examplePath = $apiDir . '/.env.example';
$dryRun = in_array('--dry-run', array_slice($argv ?? [], 1), true);

try {
    if (is_file($envPath)) {
        echo "api/.env already exists. Leaving it unchanged.\n";
        exit(0);
    }

    $example = file_get_contents($examplePath);
    if ($example === false) {
        throw new RuntimeException('Cannot read api/.env.example');
    }

    $secretLine = '/^JWT_SECRET=[ \t]*\r?$/m';
    $needsSecret = preg_match($secretLine, $example) === 1;
    $contents = $example;
    if ($needsSecret) {
        $secret = base64_encode(random_bytes(48));
        $contents = preg_replace_callback(
            $secretLine,
            static fn (array $match): string => 'JWT_SECRET=' . $secret,
            $example,
            1
        ) ?? $example;
    }

    if ($dryRun) {
        echo "Dry run. Nothing was written.\n";
        echo "Would create api/.env from api/.env.example.\n";
        echo $needsSecret ? "Would generate JWT_SECRET (value not shown).\n" : "JWT_SECRET is already set in the example.\n";
        exit(0);
    }

    // 'x' fails if the file appeared since the check above, so nothing is overwritten.
    $handle = fopen($envPath, 'xb');
    if ($handle === false) {
        throw new RuntimeException('api/.env could not be created');
    }
    fwrite($handle, $contents);
    fclose($handle);

    echo "Created api/.env from api/.env.example.\n";
    echo $needsSecret ? "Generated JWT_SECRET (value not shown).\n" : "JWT_SECRET was already set in the example.\n";
    exit(0);
} catch (\Throwable $error) {
    fwrite(STDERR, $error->getMessage() . "\n");
    exit(1);
}
