<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use app\Config\Config;
use app\Db\Database;

try {
    Config::load(dirname(__DIR__) . '/api');

    if (Config::environment() !== 'development') {
        fwrite(STDERR, "seed.php only runs when APP_ENV is development.\n");
        exit(1);
    }

    // The documented development password. It is not printed.
    $email = 'test@lodestone.test';
    $password = 'correct-horse-battery';
    $displayName = 'Test player';

    $pdo = Database::pdo();

    $find = $pdo->prepare('SELECT id FROM users WHERE email = ?');
    $find->execute([$email]);
    if ($find->fetch() !== false) {
        echo "Test account $email already exists. Its password is the documented development password.\n";
        exit(0);
    }

    $algorithm = defined('PASSWORD_ARGON2ID') ? PASSWORD_ARGON2ID : PASSWORD_DEFAULT;
    $hash = password_hash($password, $algorithm);

    $insert = $pdo->prepare(
        'INSERT INTO users (email, email_verified_at, display_name, password_hash)
         VALUES (?, CURRENT_TIMESTAMP(6), ?, ?)'
    );
    $insert->execute([$email, $displayName, $hash]);

    echo "Created test account $email, email verified. Its password is the documented development password.\n";
    exit(0);
} catch (\Throwable $error) {
    fwrite(STDERR, $error->getMessage() . "\n");
    exit(1);
}
