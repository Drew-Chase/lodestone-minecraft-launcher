<?php

declare(strict_types=1);

namespace app\Billing;

use app\Db\Database;
use PDO;
use PDOException;

/**
 * The subscriptions and stripe_events tables.
 */
final class SubscriptionStore
{
    /** Statuses in which a subscription still counts: it grants its plan or may yet do so. */
    public const LIVE = ['trialing', 'active', 'past_due', 'incomplete'];

    public static function isLive(string $status): bool
    {
        return in_array($status, self::LIVE, true);
    }

    /** @return array<string, mixed>|null */
    public function find(int $userId): ?array
    {
        $statement = Database::pdo()->prepare('SELECT * FROM subscriptions WHERE user_id = ?');
        $statement->execute([$userId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /** @return array<string, mixed>|null */
    public function findByCustomer(string $customerId): ?array
    {
        $statement = Database::pdo()->prepare('SELECT * FROM subscriptions WHERE stripe_customer_id = ?');
        $statement->execute([$customerId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /**
     * Reads the user's row and holds its lock until the transaction ends. Call inside a transaction.
     *
     * @return array<string, mixed>|null
     */
    public function lockForUpdate(int $userId): ?array
    {
        $statement = Database::pdo()->prepare('SELECT * FROM subscriptions WHERE user_id = ? FOR UPDATE');
        $statement->execute([$userId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        return $row === false ? null : $row;
    }

    /** A user who has not been deleted. */
    public function userExists(int $userId): bool
    {
        $statement = Database::pdo()->prepare('SELECT 1 FROM users WHERE id = ? AND deleted_at IS NULL');
        $statement->execute([$userId]);

        return $statement->fetchColumn() !== false;
    }

    /** Inserts the user's row with status none when it is missing. */
    public function ensureRow(int $userId): void
    {
        $statement = Database::pdo()->prepare(
            "INSERT INTO subscriptions (user_id, status) VALUES (?, 'none') ON DUPLICATE KEY UPDATE user_id = user_id"
        );
        $statement->execute([$userId]);
    }

    /**
     * Stores the customer id unless the row already has one, and returns the id the row holds afterwards.
     * A concurrent checkout that stored its customer first keeps that one.
     */
    public function attachCustomer(int $userId, string $customerId): string
    {
        $this->ensureRow($userId);

        $statement = Database::pdo()->prepare(
            'UPDATE subscriptions SET stripe_customer_id = ? WHERE user_id = ? AND stripe_customer_id IS NULL'
        );
        $statement->execute([$customerId, $userId]);

        return (string) $this->find($userId)['stripe_customer_id'];
    }

    /**
     * Replaces the row's subscription and its state. The caller decides whether to write, under the row lock.
     *
     * @param array{stripe_subscription_id: string, status: string, plan: ?string, billing_interval: ?string,
     *     trial_ends_at: ?string, current_period_end: ?string, cancel_at_period_end: int} $columns
     */
    public function saveSubscription(int $userId, array $columns): void
    {
        $statement = Database::pdo()->prepare(
            'UPDATE subscriptions
             SET stripe_subscription_id = ?, status = ?, plan = ?, billing_interval = ?,
                 trial_ends_at = ?, current_period_end = ?, cancel_at_period_end = ?
             WHERE user_id = ?'
        );
        $statement->execute([
            $columns['stripe_subscription_id'],
            $columns['status'],
            $columns['plan'],
            $columns['billing_interval'],
            $columns['trial_ends_at'],
            $columns['current_period_end'],
            $columns['cancel_at_period_end'],
            $userId,
        ]);
    }

    /**
     * Records a Stripe event in the current transaction. Returns false when the event id is already recorded
     * (SQLSTATE 23000), so the caller treats the event as handled.
     */
    public function recordEvent(string $eventId, string $type): bool
    {
        try {
            $statement = Database::pdo()->prepare('INSERT INTO stripe_events (event_id, type) VALUES (?, ?)');
            $statement->execute([$eventId, $type]);
        } catch (PDOException $error) {
            if ((string) $error->getCode() === '23000') {
                return false;
            }

            throw $error;
        }

        return true;
    }
}
