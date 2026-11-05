<?php

declare(strict_types=1);

namespace app\Db;

use app\Config\Config;
use PDO;
use PDOException;

final class Database
{
    private static ?PDO $pdo = null;

    public static function pdo(): PDO
    {
        if (self::$pdo === null) {
            $host = Config::need('DB_HOST');
            $port = Config::get('DB_PORT', '3306');
            $name = Config::need('DB_NAME');
            $user = Config::need('DB_USER');
            $password = Config::get('DB_PASSWORD', '');

            $pdo = new PDO(
                "mysql:host=$host;port=$port;dbname=$name;charset=utf8mb4",
                $user,
                $password,
                [
                    PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
                    PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
                    PDO::ATTR_EMULATE_PREPARES => false,
                    PDO::ATTR_TIMEOUT => 5,
                ]
            );

            // Every stored time is UTC.
            $pdo->exec("SET time_zone = '+00:00'");
            self::$pdo = $pdo;
        }

        return self::$pdo;
    }

    /** MySQL error 1213: the server rolled the transaction back to break a deadlock. */
    private const DEADLOCK_ERROR = 1213;

    /**
     * Runs $work inside one transaction and returns its result.
     * Nested transactions are not supported: a call inside another transaction fails at BEGIN.
     *
     * A deadlock rolls the attempt back and runs $work once more in a new transaction. Any other failure is rethrown.
     * $work may run twice, so anything it does outside the database is repeated too.
     */
    public static function transaction(callable $work): mixed
    {
        $pdo = self::pdo();

        try {
            return self::attempt($pdo, $work);
        } catch (PDOException $error) {
            if (!self::isDeadlock($error)) {
                throw $error;
            }
        }

        return self::attempt($pdo, $work);
    }

    /**
     * Runs $work in the transaction the caller already has, or in a new one when there is none.
     * PDO cannot nest a transaction, so a caller that is inside one must not open another.
     */
    public static function atomic(callable $work): mixed
    {
        $pdo = self::pdo();

        if ($pdo->inTransaction()) {
            return $work($pdo);
        }

        return self::transaction($work);
    }

    /** Begins one transaction, runs $work, and commits. A failure rolls the transaction back and is rethrown. */
    private static function attempt(PDO $pdo, callable $work): mixed
    {
        $pdo->beginTransaction();

        try {
            $result = $work($pdo);
            $pdo->commit();

            return $result;
        } catch (\Throwable $error) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }

            throw $error;
        }
    }

    private static function isDeadlock(PDOException $error): bool
    {
        return (int) ($error->errorInfo[1] ?? 0) === self::DEADLOCK_ERROR;
    }

    /** Drops the cached connection. Tests only. */
    public static function reset(): void
    {
        self::$pdo = null;
    }
}
