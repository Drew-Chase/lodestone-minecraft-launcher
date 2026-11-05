<?php

declare(strict_types=1);

namespace tests\Billing;

use app\Billing\PriceMap;
use app\Config\Config;

final class PriceMapTest extends BillingTestCase
{
    public function test_each_configured_price_resolves_to_its_plan_and_interval(): void
    {
        $map = PriceMap::fromConfig();
        $this->assertNotNull($map);

        $expected = [
            'price_test_solo_monthly' => ['plan' => 'solo', 'interval' => 'monthly'],
            'price_test_solo_yearly' => ['plan' => 'solo', 'interval' => 'yearly'],
            'price_test_friends_monthly' => ['plan' => 'friends', 'interval' => 'monthly'],
            'price_test_friends_yearly' => ['plan' => 'friends', 'interval' => 'yearly'],
            'price_test_unlimited_monthly' => ['plan' => 'unlimited', 'interval' => 'monthly'],
            'price_test_unlimited_yearly' => ['plan' => 'unlimited', 'interval' => 'yearly'],
        ];

        foreach ($expected as $priceId => $match) {
            $this->assertSame($match, $map->lookup($priceId), $priceId);
        }

        $this->assertSame('price_test_friends_yearly', $map->priceId('friends', 'yearly'));
    }

    public function test_an_unknown_price_id_resolves_to_null(): void
    {
        $map = PriceMap::fromConfig();
        $this->assertNotNull($map);

        $this->assertNull($map->lookup('price_not_configured'));
        $this->assertNull($map->lookup(''));
    }

    public function test_the_map_is_null_when_one_price_is_missing_or_empty(): void
    {
        Config::set('STRIPE_PRICE_UNLIMITED_YEARLY', '');

        $this->assertNull(PriceMap::fromConfig());
    }
}
