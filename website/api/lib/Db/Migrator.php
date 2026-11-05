<?php

declare(strict_types=1);

namespace app\Db;

use PDO;

/**
 * Applies the numbered SQL files in a directory, in name order, once each.
 * Each file is not run in a transaction, because DDL commits implicitly.
 */
final class Migrator
{
    private const FILE_PATTERN = '/^\d{4}_[a-z0-9_]+\.sql$/';

    public function __construct(
        private readonly PDO $pdo,
        private readonly string $directory,
    ) {
    }

    /**
     * @return list<string> File names not yet recorded, ascending.
     */
    public function pending(): array
    {
        $this->ensureTable();

        $applied = array_flip($this->applied());

        $files = scandir($this->directory);
        if ($files === false) {
            throw new \RuntimeException("Cannot read migrations directory {$this->directory}");
        }

        $files = array_values(array_filter(
            $files,
            static fn (string $name): bool => preg_match(self::FILE_PATTERN, $name) === 1
        ));
        sort($files, SORT_STRING);

        return array_values(array_filter(
            $files,
            static fn (string $name): bool => !isset($applied[$name])
        ));
    }

    /**
     * @return list<string> The names applied by this call, in order.
     */
    public function migrate(): array
    {
        $applied = [];

        foreach ($this->pending() as $name) {
            $path = $this->directory . DIRECTORY_SEPARATOR . $name;
            $sql = file_get_contents($path);
            if ($sql === false) {
                throw new \RuntimeException("Migration $name failed: cannot read $path");
            }

            try {
                foreach (self::split($sql) as $statement) {
                    $this->pdo->exec($statement);
                }
            } catch (\Throwable $error) {
                throw new \RuntimeException("Migration $name failed: " . $error->getMessage(), 0, $error);
            }

            $record = $this->pdo->prepare('INSERT INTO schema_migrations (version) VALUES (?)');
            $record->execute([$name]);
            $applied[] = $name;
        }

        return $applied;
    }

    /**
     * Drops comment lines, then splits on each semicolon that ends a line.
     *
     * @return list<string>
     */
    public static function split(string $sql): array
    {
        $kept = [];
        foreach (preg_split('/\r\n|\r|\n/', $sql) ?: [] as $line) {
            if (str_starts_with(trim($line), '--')) {
                continue;
            }
            $kept[] = $line;
        }

        $statements = [];
        foreach (preg_split('/;(?=\n|\z)/', implode("\n", $kept)) ?: [] as $part) {
            $statement = trim($part);
            if ($statement !== '') {
                $statements[] = $statement;
            }
        }

        return $statements;
    }

    private function ensureTable(): void
    {
        $this->pdo->exec(<<<'SQL'
CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(191) NOT NULL PRIMARY KEY,
    applied_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
SQL);
    }

    /**
     * @return list<string>
     */
    private function applied(): array
    {
        $rows = $this->pdo->query('SELECT version FROM schema_migrations')->fetchAll(PDO::FETCH_COLUMN);

        return array_values(array_map('strval', $rows));
    }
}
