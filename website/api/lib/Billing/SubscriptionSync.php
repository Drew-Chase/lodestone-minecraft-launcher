<?php

declare(strict_types=1);

namespace app\Billing;

use DateTimeImmutable;
use DateTimeZone;

/**
 * Turns a Stripe subscription object into the columns of a subscriptions row.
 */
final class SubscriptionSync
{
    private function __construct()
    {
    }

    /**
     * @param array<string, mixed> $subscription The subscription object, as an array.
     * @return array{stripe_subscription_id: string, status: string, plan: ?string, billing_interval: ?string,
     *     trial_ends_at: ?string, current_period_end: ?string, cancel_at_period_end: int}
     */
    public static function columns(array $subscription, string $status, PriceMap $prices): array
    {
        $item = $subscription['items']['data'][0] ?? [];
        $priceId = $item['price']['id'] ?? null;
        $match = is_string($priceId) ? $prices->lookup($priceId) : null;

        // Newer Stripe API versions put the period on the item rather than on the subscription.
        $periodEnd = $subscription['current_period_end'] ?? $item['current_period_end'] ?? null;

        return [
            'stripe_subscription_id' => (string) $subscription['id'],
            'status' => $status,
            'plan' => $match['plan'] ?? null,
            'billing_interval' => $match['interval'] ?? null,
            'trial_ends_at' => self::utc($subscription['trial_end'] ?? null),
            'current_period_end' => self::utc($periodEnd),
            'cancel_at_period_end' => ($subscription['cancel_at_period_end'] ?? false) ? 1 : 0,
        ];
    }

    /** A Unix timestamp in seconds as a UTC DATETIME(6) string, or null. */
    public static function utc(mixed $seconds): ?string
    {
        if (!is_int($seconds)) {
            return null;
        }

        return (new DateTimeImmutable('@' . $seconds))
            ->setTimezone(new DateTimeZone('UTC'))
            ->format('Y-m-d H:i:s.u');
    }
}
