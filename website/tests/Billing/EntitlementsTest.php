<?php

declare(strict_types=1);

namespace tests\Billing;

use app\Billing\Entitlements;

final class EntitlementsTest extends BillingTestCase
{
    public function test_a_user_with_no_row_is_on_the_free_plan(): void
    {
        $userId = $this->createUser('none@example.com');

        $subscription = Entitlements::for($userId);

        $this->assertSame('free', $subscription['plan']);
        $this->assertSame('none', $subscription['status']);
        $this->assertNull($subscription['interval']);
        $this->assertSame(0, $subscription['storage_bytes']);
        $this->assertSame(1, $subscription['devices']);
        $this->assertSame(0, $subscription['shares']);
    }

    public function test_a_trialing_solo_row_gets_the_solo_limits(): void
    {
        $userId = $this->createUser('trial@example.com');
        $this->insertSubscription($userId, [
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'trialing',
            'trial_ends_at' => '2030-03-17 17:46:40.000000',
        ]);

        $subscription = Entitlements::for($userId);

        $this->assertSame('solo', $subscription['plan']);
        $this->assertSame('Solo Sync', $subscription['name']);
        $this->assertSame('trialing', $subscription['status']);
        $this->assertSame('monthly', $subscription['interval']);
        $this->assertSame(100_000_000_000, $subscription['storage_bytes']);
        $this->assertSame(5, $subscription['devices']);
        $this->assertSame(0, $subscription['shares']);
        $this->assertSame('2030-03-17T17:46:40+00:00', $subscription['trial_ends_at']);
    }

    public function test_a_canceled_solo_row_falls_back_to_free_and_keeps_its_status(): void
    {
        $userId = $this->createUser('canceled@example.com');
        $this->insertSubscription($userId, [
            'plan' => 'solo',
            'billing_interval' => 'yearly',
            'status' => 'canceled',
        ]);

        $subscription = Entitlements::for($userId);

        $this->assertSame('free', $subscription['plan']);
        $this->assertSame('canceled', $subscription['status']);
        $this->assertSame(0, $subscription['storage_bytes']);
    }

    public function test_a_past_due_friends_row_keeps_the_friends_limits(): void
    {
        $userId = $this->createUser('pastdue@example.com');
        $this->insertSubscription($userId, [
            'plan' => 'friends',
            'billing_interval' => 'yearly',
            'status' => 'past_due',
            'cancel_at_period_end' => 1,
        ]);

        $subscription = Entitlements::for($userId);

        $this->assertSame('friends', $subscription['plan']);
        $this->assertSame('past_due', $subscription['status']);
        $this->assertSame(250_000_000_000, $subscription['storage_bytes']);
        $this->assertSame(5, $subscription['devices']);
        $this->assertSame(5, $subscription['shares']);
        $this->assertTrue($subscription['cancel_at_period_end']);
    }

    public function test_an_unlimited_row_has_no_device_limit(): void
    {
        $userId = $this->createUser('unlimited@example.com');
        $this->insertSubscription($userId, [
            'plan' => 'unlimited',
            'billing_interval' => 'monthly',
            'status' => 'active',
        ]);

        $subscription = Entitlements::for($userId);

        $this->assertSame('unlimited', $subscription['plan']);
        $this->assertNull($subscription['devices']);
        $this->assertNull($subscription['shares']);
        $this->assertSame(500_000_000_000, $subscription['storage_bytes']);
    }
}
