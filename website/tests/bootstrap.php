<?php

declare(strict_types=1);

require dirname(__DIR__) . '/vendor/autoload.php';

use app\Config\Config;
use app\Db\Database;
use app\Db\Migrator;

Config::load(dirname(__DIR__) . '/api');

// The suite truncates tables, so it must never touch a database that is not a test database.
if (!str_ends_with(Config::need('DB_NAME'), '_test')) {
    throw new \RuntimeException('Refusing to run tests against a database whose name does not end in _test');
}

// Local servers only, so a remote host in the environment can never be truncated.
$testHost = Config::need('DB_HOST');
if (!in_array($testHost, ['127.0.0.1', 'localhost', '::1'], true)) {
    throw new \RuntimeException("Refusing to run tests against a remote database host: {$testHost}");
}

// Say where the suite is pointed, so an environment override is visible in the output.
fwrite(STDERR, 'Test database: ' . $testHost . ':' . Config::get('DB_PORT', '3306') . '/' . Config::need('DB_NAME') . "\n");

function lodestone_migrate_once(): void
{
    static $done = false;
    if ($done) {
        return;
    }
    $done = true;

    (new Migrator(Database::pdo(), dirname(__DIR__) . '/api/migrations'))->migrate();
}

lodestone_migrate_once();
