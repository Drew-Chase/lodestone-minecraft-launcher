<?php

declare(strict_types=1);

// Scans a built folder for production secrets. CI runs this on master builds after the
// deploy folder is assembled. Usage, from website/:
//   php scripts/check-dist-secrets.php dist
//
// The check fails closed: no secrets set, a secret too short to check, or an unreadable
// file all fail the run. Only the names of the secrets and the relative paths of matching
// files are printed, never the values.

const SECRET_NAMES = [
    'DB_PASSWORD', 'JWT_SECRET', 'SMTP_PASSWORD',
    'GOOGLE_CLIENT_SECRET', 'MICROSOFT_CLIENT_SECRET', 'GITHUB_CLIENT_SECRET',
    'STRIPE_SECRET_KEY', 'STRIPE_WEBHOOK_SECRET',
    'R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY',
];

const MIN_LENGTH = 8;
const CHUNK_BYTES = 8 * 1024 * 1024;

// The one file that is allowed to hold secrets, relative to the scanned folder.
const ALLOWED_FILE = 'api/config/production.php';

if ($argc !== 2) {
    fwrite(STDERR, "Usage: php scripts/check-dist-secrets.php <folder>\n");
    exit(1);
}

$folder = $argv[1];
if (!is_dir($folder)) {
    fwrite(STDERR, "The folder to scan does not exist: {$folder}\n");
    exit(1);
}

$root = rtrim(str_replace('\\', '/', realpath($folder)), '/');

// Collect the secrets to look for. Unset or empty variables are skipped. A value that is
// too short to match reliably fails the run, and only its name is printed.
$secrets = [];
foreach (SECRET_NAMES as $name) {
    $value = getenv($name);
    if ($value === false || $value === '') {
        continue;
    }
    if (strlen($value) < MIN_LENGTH) {
        fwrite(STDERR, "FAILED {$name}: too short to check reliably (fewer than " . MIN_LENGTH . " characters).\n");
        exit(1);
    }
    $secrets[$name] = $value;
}

if ($secrets === []) {
    fwrite(STDERR, "No secrets to check. Refusing to pass an empty scan.\n");
    exit(1);
}

echo 'Checking ' . count($secrets) . " secret(s) against the files in {$folder}.\n";

$iterator = new RecursiveIteratorIterator(
    new RecursiveDirectoryIterator($root, FilesystemIterator::SKIP_DOTS)
);

$matches = 0;
$unreadable = 0;
foreach ($iterator as $fileInfo) {
    if (!$fileInfo->isFile()) {
        continue;
    }

    $path = str_replace('\\', '/', $fileInfo->getPathname());
    $relative = substr($path, strlen($root) + 1);

    if ($relative === ALLOWED_FILE) {
        continue;
    }

    $handle = fopen($path, 'rb');
    if ($handle === false) {
        fwrite(STDERR, "ERROR: could not read {$relative}.\n");
        $unreadable++;
        continue;
    }

    // Stream the file in chunks. Each secret keeps the last (length - 1) bytes it has seen,
    // so a match that straddles two chunks is still found.
    $carry = [];
    $found = [];
    foreach ($secrets as $name => $value) {
        $carry[$name] = '';
    }

    while (true) {
        $chunk = fread($handle, CHUNK_BYTES);
        if ($chunk === false) {
            fwrite(STDERR, "ERROR: could not read {$relative}.\n");
            $unreadable++;
            break;
        }
        if ($chunk === '') {
            break;
        }

        foreach ($secrets as $name => $value) {
            if (isset($found[$name])) {
                continue;
            }
            $window = $carry[$name] . $chunk;
            if (strpos($window, $value) !== false) {
                $found[$name] = true;
                echo "FOUND {$name} in {$relative}\n";
                $matches++;
                continue;
            }
            $keep = strlen($value) - 1;
            $carry[$name] = $keep > 0 ? substr($window, -$keep) : '';
        }
    }

    fclose($handle);
}

if ($matches > 0 || $unreadable > 0) {
    fwrite(STDERR, "{$matches} secret match(es) found, {$unreadable} file(s) could not be read.\n");
    exit(1);
}

echo "No secrets found in the build output.\n";
exit(0);
