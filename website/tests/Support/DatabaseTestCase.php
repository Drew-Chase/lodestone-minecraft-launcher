<?php

declare(strict_types=1);

namespace tests\Support;

use app\Config\Config;
use app\Db\Database;
use PDO;
use PHPUnit\Framework\TestCase;

abstract class DatabaseTestCase extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        // Config may have been reset by an earlier test, so load it again. Refuse to truncate
        // anything that is not a test database, even if the values came from a .env file.
        Config::load(dirname(__DIR__, 2) . '/api');
        if (!str_ends_with(Config::need('DB_NAME'), '_test')) {
            throw new \RuntimeException('Refusing to truncate a database whose name does not end in _test');
        }

        // Every table is emptied, so a new migration needs no change here. The migration ledger keeps its rows.
        $pdo = $this->pdo();
        $tables = $pdo->query(
            "SELECT table_name FROM information_schema.tables
             WHERE table_schema = DATABASE() AND table_name <> 'schema_migrations'"
        )->fetchAll(PDO::FETCH_COLUMN);

        $pdo->exec('SET FOREIGN_KEY_CHECKS = 0');
        try {
            foreach ($tables as $table) {
                $pdo->exec("TRUNCATE TABLE `$table`");
            }
        } finally {
            $pdo->exec('SET FOREIGN_KEY_CHECKS = 1');
        }
    }

    protected function pdo(): PDO
    {
        return Database::pdo();
    }
}
