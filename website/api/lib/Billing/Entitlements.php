<?php

declare(strict_types=1);

namespace app\Billing;

use app\Db\Database;
use DateTimeImmutable;
use DateTimeInterface;
use DateTimeZone;
use PDO;

/**
 * What a user may use right now, read from the local subscriptions row. Limit checks use this, never Stripe.
 */
final class Entitlements
{
    /** past_due keeps the paid allowances while Stripe retries the payment. */
    private const PAID_STATUSES = ['trialing', 'active', 'past_due'];

    private function __construct()
    {
    }

    /**
     * @return array{plan: string, name: string, status: string, interval: ?string, storage_bytes: int,
     *     devices: ?int, shares: ?int, trial_ends_at: ?string, current_period_end: ?string, cancel_at_period_end: bool}
     */
    public static function for(int $userId): array
    {
        $statement = Database::pdo()->prepare(
            'SELECT plan, billing_interval, status, trial_ends_at, current_period_end, cancel_at_period_end
             FROM subscriptions WHERE user_id = ?'
        );
        $statement->execute([$userId]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);

        if ($row === false) {
            return self::free('none', null, null);
        }

        $paid = $row['plan'] !== null
            && Plans::isPaid((string) $row['plan'])
            && in_array($row['status'], self::PAID_STATUSES, true);

        if (!$paid) {
            return self::free((string) $row['status'], self::iso($row['trial_ends_at']), self::iso($row['current_period_end']));
        }

        $plan = Plans::get((string) $row['plan']);

        return [
            'plan' => (string) $row['plan'],
            'name' => $plan['name'],
            'status' => (string) $row['status'],
            'interval' => $row['billing_interval'] === null ? null : (string) $row['billing_interval'],
            'storage_bytes' => $plan['storage_bytes'],
            'devices' => $plan['devices'],
            'shares' => $plan['shares'],
            'trial_ends_at' => self::iso($row['trial_ends_at']),
            'current_period_end' => self::iso($row['current_period_end']),
            'cancel_at_period_end' => (bool) $row['cancel_at_period_end'],
        ];
    }

    /** @return array{plan: string, name: string, status: string, interval: ?string, storage_bytes: int, devices: ?int, shares: ?int, trial_ends_at: ?string, current_period_end: ?string, cancel_at_period_end: bool} */
    private static function free(string $status, ?string $trialEndsAt, ?string $currentPeriodEnd): array
    {
        $plan = Plans::get(Plans::FREE);

        return [
            'plan' => Plans::FREE,
            'name' => $plan['name'],
            'status' => $status,
            'interval' => null,
            'storage_bytes' => $plan['storage_bytes'],
            'devices' => $plan['devices'],
            'shares' => $plan['shares'],
            'trial_ends_at' => $trialEndsAt,
            'current_period_end' => $currentPeriodEnd,
            'cancel_at_period_end' => false,
        ];
    }

    /** A stored UTC time as ISO 8601, or null. */
    private static function iso(?string $value): ?string
    {
        if ($value === null) {
            return null;
        }

        return (new DateTimeImmutable($value, new DateTimeZone('UTC')))->format(DateTimeInterface::ATOM);
    }
}
