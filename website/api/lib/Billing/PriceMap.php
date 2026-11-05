<?php

declare(strict_types=1);

namespace app\Billing;

use app\Config\Config;

/**
 * Maps Stripe price ids to the plan and interval each one sells. Built from the STRIPE_PRICE_* keys.
 */
final class PriceMap
{
    /**
     * @param array<string, array{plan: string, interval: string}> $byPrice
     * @param array<string, array<string, string>> $byPlan plan => interval => price id
     */
    private function __construct(
        private readonly array $byPrice,
        private readonly array $byPlan,
    ) {
    }

    /** The map, or null when any of the six price keys is missing or empty. */
    public static function fromConfig(): ?self
    {
        $byPrice = [];
        $byPlan = [];

        foreach (Plans::paidIds() as $plan) {
            foreach (Plans::get($plan)['intervals'] as $interval) {
                $priceId = Config::get(self::configKey($plan, $interval));
                if ($priceId === null) {
                    return null;
                }

                $byPrice[$priceId] = ['plan' => $plan, 'interval' => $interval];
                $byPlan[$plan][$interval] = $priceId;
            }
        }

        return new self($byPrice, $byPlan);
    }

    public static function configKey(string $plan, string $interval): string
    {
        return 'STRIPE_PRICE_' . strtoupper($plan) . '_' . strtoupper($interval);
    }

    /** @return array{plan: string, interval: string}|null Null for a price the map does not know. */
    public function lookup(string $priceId): ?array
    {
        return $this->byPrice[$priceId] ?? null;
    }

    public function priceId(string $plan, string $interval): ?string
    {
        return $this->byPlan[$plan][$interval] ?? null;
    }
}
