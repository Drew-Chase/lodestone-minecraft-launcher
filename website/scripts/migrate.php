<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use app\Config\Config;
use app\Db\Database;
use app\Db\Migrator;

try {
    $apiDir = dirname(__DIR__) . '/api';
    Config::load($apiDir);
    // Say where the migrations go, so an environment override is visible.
    fwrite(STDERR, 'Database: ' . Config::need('DB_HOST') . ':' . Config::get('DB_PORT', '3306') . '/' . Config::need('DB_NAME') . "\n");

    $migrator = new Migrator(Database::pdo(), $apiDir . '/migrations');
    $options = array_slice($argv ?? [], 1);

    if (in_array('--status', $options, true)) {
        $pending = $migrator->pending();
        if ($pending === []) {
            echo "Nothing to apply.\n";
        }
        foreach ($pending as $name) {
            echo "$name\n";
        }
        exit(0);
    }

    $applied = $migrator->migrate();
    if ($applied === []) {
        echo "Nothing to apply.\n";
    }
    foreach ($applied as $name) {
        echo "Applied $name\n";
    }
    exit(0);
} catch (\Throwable $error) {
    fwrite(STDERR, $error->getMessage() . "\n");
    exit(1);
}
