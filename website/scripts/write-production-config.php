<?php

declare(strict_types=1);

// Writes api/config/production.php from environment variables. CI runs this on master
// builds, with the values taken from GitHub secrets. Usage, from website/:
//   php scripts/write-production-config.php
//
// Values are written with var_export, so no value can break out of its string. Only the
// names of the keys are printed, never their values.

if (getenv('GITHUB_ACTIONS') !== 'true') {
    fwrite(STDERR, "Refusing to write the production config outside GitHub Actions (GITHUB_ACTIONS is not 'true').\n");
    exit(1);
}

const REQUIRED = ['APP_URL', 'JWT_SECRET', 'DB_HOST', 'DB_PORT', 'DB_NAME', 'DB_USER', 'DB_PASSWORD'];

const OPTIONAL = [
    'MAIL_FROM', 'SMTP_HOST', 'SMTP_PORT', 'SMTP_USERNAME', 'SMTP_PASSWORD',
    'GOOGLE_CLIENT_ID', 'GOOGLE_CLIENT_SECRET',
    'MICROSOFT_CLIENT_ID', 'MICROSOFT_CLIENT_SECRET',
    'GITHUB_CLIENT_ID', 'GITHUB_CLIENT_SECRET',
    'STRIPE_SECRET_KEY', 'STRIPE_PUBLISHABLE_KEY', 'STRIPE_WEBHOOK_SECRET',
    'STRIPE_PRICE_SOLO_MONTHLY', 'STRIPE_PRICE_SOLO_YEARLY',
    'STRIPE_PRICE_FRIENDS_MONTHLY', 'STRIPE_PRICE_FRIENDS_YEARLY',
    'STRIPE_PRICE_UNLIMITED_MONTHLY', 'STRIPE_PRICE_UNLIMITED_YEARLY',
    'R2_ENDPOINT', 'R2_BUCKET', 'R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY',
];

const MIN_JWT_SECRET_LENGTH = 32;

/** @return string|false */
function env(string $name)
{
    return getenv($name);
}

$config = ['APP_ENV' => 'production'];

$missing = [];
foreach (REQUIRED as $name) {
    $value = env($name);
    if ($value === false || $value === '') {
        $missing[] = $name;
        continue;
    }
    $config[$name] = $value;
}

if ($missing !== []) {
    foreach ($missing as $name) {
        fwrite(STDERR, "Missing required environment variable: {$name}\n");
    }
    exit(1);
}

if (strlen($config['JWT_SECRET']) < MIN_JWT_SECRET_LENGTH) {
    fwrite(STDERR, 'JWT_SECRET must be at least ' . MIN_JWT_SECRET_LENGTH . " characters long.\n");
    exit(1);
}

foreach (OPTIONAL as $name) {
    $value = env($name);
    $config[$name] = $value === false ? '' : $value;
}

$root = dirname(__DIR__);
$dir = $root . '/api/config';
if (!is_dir($dir) && !mkdir($dir, 0775, true) && !is_dir($dir)) {
    fwrite(STDERR, "Could not create the config folder api/config.\n");
    exit(1);
}

$file = $dir . '/production.php';
$source = "<?php\n\ndeclare(strict_types=1);\n\nreturn " . var_export($config, true) . ";\n";

if (file_put_contents($file, $source, LOCK_EX) === false) {
    fwrite(STDERR, "Could not write api/config/production.php.\n");
    exit(1);
}

echo "Wrote api/config/production.php with these keys:\n";
foreach (array_keys($config) as $name) {
    echo "  {$name}\n";
}
exit(0);
