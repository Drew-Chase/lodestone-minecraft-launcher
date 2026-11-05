<?php

declare(strict_types=1);

namespace tests\Db;

use app\Db\Migrator;
use PDO;
use RuntimeException;
use tests\Support\DatabaseTestCase;

final class MigratorTest extends DatabaseTestCase
{
    private string $suffix;
    private string $table;
    private string $good;
    private string $bad;
    private string $dir;

    protected function setUp(): void
    {
        parent::setUp();

        $this->suffix = bin2hex(random_bytes(4));
        $this->table = 'tmp_one_' . $this->suffix;
        $this->good = '0001_one_' . $this->suffix . '.sql';
        $this->bad = '0002_bad_' . $this->suffix . '.sql';
        $this->dir = sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'lodestone-migrations-' . $this->suffix;

        mkdir($this->dir, 0777, true);
        file_put_contents(
            $this->dir . DIRECTORY_SEPARATOR . $this->good,
            "CREATE TABLE {$this->table} (id INT NOT NULL PRIMARY KEY) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;\n"
        );
        file_put_contents(
            $this->dir . DIRECTORY_SEPARATOR . $this->bad,
            "CREATE TABL this_is_not_sql;\n"
        );
    }

    protected function tearDown(): void
    {
        if (isset($this->table)) {
            $this->pdo()->exec("DROP TABLE IF EXISTS `{$this->table}`");

            $delete = $this->pdo()->prepare('DELETE FROM schema_migrations WHERE version IN (?, ?)');
            $delete->execute([$this->good, $this->bad]);
        }

        if (isset($this->dir) && is_dir($this->dir)) {
            foreach (scandir($this->dir) ?: [] as $name) {
                if ($name !== '.' && $name !== '..') {
                    unlink($this->dir . DIRECTORY_SEPARATOR . $name);
                }
            }
            rmdir($this->dir);
        }

        parent::tearDown();
    }

    public function testFirstRunAppliesOnlyTheGoodFileAndNamesTheBadOne(): void
    {
        $migrator = new Migrator($this->pdo(), $this->dir);

        try {
            $migrator->migrate();
            self::fail('The bad migration should have failed.');
        } catch (RuntimeException $error) {
            self::assertStringContainsString($this->bad, $error->getMessage());
        }

        $tables = $this->pdo()->query("SHOW TABLES LIKE '{$this->table}'")->fetchAll(PDO::FETCH_COLUMN);
        self::assertSame([$this->table], $tables);

        $recorded = $this->recorded();
        self::assertContains($this->good, $recorded);
        self::assertNotContains($this->bad, $recorded);

        self::assertSame([$this->bad], $migrator->pending());
    }

    public function testSplitDropsCommentLinesAndSplitsOnSemicolonsAtLineEnd(): void
    {
        $sql = "-- header; not a split\n"
            . "CREATE TABLE a (id INT) ENGINE=InnoDB;\n"
            . "  -- indented comment; also dropped\n"
            . "INSERT INTO a VALUES ('x;y');\n"
            . "SELECT 1\n"
            . ";\n"
            . "\n";

        self::assertSame(
            ['CREATE TABLE a (id INT) ENGINE=InnoDB', "INSERT INTO a VALUES ('x;y')", 'SELECT 1'],
            Migrator::split($sql)
        );
    }

    /**
     * @return list<string>
     */
    private function recorded(): array
    {
        $statement = $this->pdo()->prepare('SELECT version FROM schema_migrations WHERE version IN (?, ?)');
        $statement->execute([$this->good, $this->bad]);

        return array_values(array_map('strval', $statement->fetchAll(PDO::FETCH_COLUMN)));
    }
}
