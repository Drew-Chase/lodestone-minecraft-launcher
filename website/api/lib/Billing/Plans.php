<?php

declare(strict_types=1);

namespace app\Billing;

/**
 * The plans and the allowances each one gives. Storage is in bytes, with decimal gigabytes (1 GB is 1,000,000,000 bytes).
 * A null device or share limit means no limit.
 */
final class Plans
{
    public const FREE = 'free';

    private const FREE_PLAN = [
        'name' => 'Free',
        'storage_bytes' => 0,
        'devices' => 1,
        'shares' => 0,
        'intervals' => [],
    ];

    /** @var array<string, array{name: string, storage_bytes: int, devices: ?int, shares: ?int, intervals: list<string>}> */
    private const PAID = [
        'solo' => [
            'name' => 'Solo Sync',
            'storage_bytes' => 100_000_000_000,
            'devices' => 5,
            'shares' => 0,
            'intervals' => ['monthly', 'yearly'],
        ],
        'friends' => [
            'name' => 'Friends',
            'storage_bytes' => 250_000_000_000,
            'devices' => 5,
            'shares' => 5,
            'intervals' => ['monthly', 'yearly'],
        ],
        'unlimited' => [
            'name' => 'Unlimited',
            'storage_bytes' => 500_000_000_000,
            'devices' => null,
            'shares' => null,
            'intervals' => ['monthly', 'yearly'],
        ],
    ];

    private function __construct()
    {
    }

    /** @return list<string> The paid plan identifiers, in display order. */
    public static function paidIds(): array
    {
        return array_keys(self::PAID);
    }

    public static function isPaid(string $plan): bool
    {
        return isset(self::PAID[$plan]);
    }

    public static function isInterval(string $interval): bool
    {
        return $interval === 'monthly' || $interval === 'yearly';
    }

    /**
     * @return array{name: string, storage_bytes: int, devices: ?int, shares: ?int, intervals: list<string>}
     * @throws \InvalidArgumentException for an identifier that is neither free nor a paid plan
     */
    public static function get(string $plan): array
    {
        if ($plan === self::FREE) {
            return self::FREE_PLAN;
        }

        if (!isset(self::PAID[$plan])) {
            throw new \InvalidArgumentException('Unknown plan');
        }

        return self::PAID[$plan];
    }
}
