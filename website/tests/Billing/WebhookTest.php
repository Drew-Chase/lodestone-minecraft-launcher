<?php

declare(strict_types=1);

namespace tests\Billing;

use app\Config\Config;

final class WebhookTest extends BillingTestCase
{
    private const PATH = '/api/billing/webhook';
    private const CUSTOMER = 'cus_test_1';
    private const SUBSCRIPTION = 'sub_test_1';
    private const OTHER_SUBSCRIPTION = 'sub_test_2';

    public function test_a_validly_signed_event_is_accepted(): void
    {
        $userId = $this->createUser('signed@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);

        $subscription = $this->remember($this->subscription('active', 'price_test_solo_monthly'));
        $payload = $this->event('evt_signed_1', 'customer.subscription.updated', $subscription);

        $response = $this->deliver($payload, $this->signature($payload));

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['status' => 'processed'], $this->body($response));
    }

    public function test_a_wrong_secret_gives_400_invalid_signature(): void
    {
        $payload = $this->event('evt_wrong_secret', 'customer.subscription.updated', $this->subscription('active', 'price_test_solo_monthly'));

        $response = $this->deliver($payload, $this->signature($payload, secret: 'whsec_someone_else'));

        $this->assertError($response, 400, 'invalid_signature');
    }

    public function test_a_timestamp_more_than_300_seconds_old_gives_400(): void
    {
        $payload = $this->event('evt_stale', 'customer.subscription.updated', $this->subscription('active', 'price_test_solo_monthly'));

        $response = $this->deliver($payload, $this->signature($payload, timestamp: time() - 301));

        $this->assertError($response, 400, 'invalid_signature');
    }

    public function test_a_missing_signature_header_gives_400(): void
    {
        $payload = $this->event('evt_no_header', 'customer.subscription.updated', $this->subscription('active', 'price_test_solo_monthly'));

        $response = $this->deliver($payload, '');

        $this->assertError($response, 400, 'invalid_signature');
    }

    public function test_the_same_event_twice_is_a_duplicate_the_second_time_and_changes_nothing(): void
    {
        $userId = $this->createUser('twice@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);

        $subscription = $this->remember($this->subscription('active', 'price_test_solo_monthly'));
        $payload = $this->event('evt_twice', 'customer.subscription.updated', $subscription);
        $header = $this->signature($payload);

        $first = $this->deliver($payload, $header);
        $this->assertSame(['status' => 'processed'], $this->body($first));
        $this->assertSame('active', $this->subscriptionRow($userId)['status']);

        $this->pdo()->prepare("UPDATE subscriptions SET status = 'canceled' WHERE user_id = ?")->execute([$userId]);

        $second = $this->deliver($payload, $header);
        $this->assertSame(200, $second->getStatusCode());
        $this->assertSame(['status' => 'duplicate'], $this->body($second));
        $this->assertSame('canceled', $this->subscriptionRow($userId)['status']);
    }

    public function test_a_subscription_event_sets_the_row_from_the_retrieved_subscription(): void
    {
        $userId = $this->createUser('sets@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);

        $subscription = $this->subscription('trialing', 'price_test_friends_yearly');
        $subscription['cancel_at_period_end'] = true;
        $this->remember($subscription);
        $payload = $this->event('evt_sets_row', 'customer.subscription.created', $subscription);

        $response = $this->deliver($payload, $this->signature($payload));

        $this->assertSame(['status' => 'processed'], $this->body($response));

        $row = $this->subscriptionRow($userId);
        $this->assertSame(self::SUBSCRIPTION, $row['stripe_subscription_id']);
        $this->assertSame('trialing', $row['status']);
        $this->assertSame('friends', $row['plan']);
        $this->assertSame('yearly', $row['billing_interval']);
        $this->assertSame(gmdate('Y-m-d H:i:s', 1900000000) . '.000000', $row['trial_ends_at']);
        $this->assertSame(gmdate('Y-m-d H:i:s', 1900600000) . '.000000', $row['current_period_end']);
        $this->assertSame(1, (int) $row['cancel_at_period_end']);
    }

    public function test_the_state_is_read_from_stripe_and_not_from_the_event_payload(): void
    {
        $userId = $this->createUser('retrieved@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);

        $this->remember($this->subscription('incomplete', 'price_test_solo_monthly'));
        $payload = $this->event('evt_payload_ignored', 'customer.subscription.updated', $this->subscription('active', 'price_test_solo_monthly'));

        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($payload, $this->signature($payload))));

        $this->assertSame('incomplete', $this->subscriptionRow($userId)['status']);
        $this->assertSame([
            ['method' => 'get', 'path' => '/v1/subscriptions/' . self::SUBSCRIPTION],
        ], $this->requests());
    }

    public function test_the_period_end_is_read_from_the_item_when_the_subscription_has_none(): void
    {
        $userId = $this->createUser('item-period@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);

        $subscription = $this->subscription('active', 'price_test_solo_monthly');
        unset($subscription['current_period_end']);
        $subscription['items']['data'][0]['current_period_end'] = 1900600000;
        $this->remember($subscription);
        $payload = $this->event('evt_item_period', 'customer.subscription.updated', $subscription);

        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($payload, $this->signature($payload))));
        $this->assertSame(gmdate('Y-m-d H:i:s', 1900600000) . '.000000', $this->subscriptionRow($userId)['current_period_end']);
    }

    public function test_a_subscription_for_an_unknown_customer_is_ignored_without_asking_stripe(): void
    {
        $payload = $this->event('evt_unknown_customer', 'customer.subscription.created', $this->subscription('active', 'price_test_solo_monthly'));

        $response = $this->deliver($payload, $this->signature($payload));

        $this->assertSame(['status' => 'ignored'], $this->body($response));
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_an_unknown_event_type_is_recorded_and_ignored(): void
    {
        $payload = $this->event('evt_unknown_type', 'customer.created', ['id' => self::CUSTOMER, 'object' => 'customer']);

        $response = $this->deliver($payload, $this->signature($payload));

        $this->assertSame(['status' => 'ignored'], $this->body($response));

        $statement = $this->pdo()->prepare('SELECT type FROM stripe_events WHERE event_id = ?');
        $statement->execute(['evt_unknown_type']);
        $this->assertSame('customer.created', $statement->fetchColumn());
    }

    public function test_checkout_completed_links_the_customer_and_applies_the_retrieved_subscription(): void
    {
        $userId = $this->createUser('linked@example.com');
        $this->remember($this->subscription('trialing', 'price_test_solo_monthly'));

        $payload = $this->event('evt_checkout_done', 'checkout.session.completed', $this->checkout($userId));

        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($payload, $this->signature($payload))));

        $row = $this->subscriptionRow($userId);
        $this->assertSame(self::CUSTOMER, $row['stripe_customer_id']);
        $this->assertSame(self::SUBSCRIPTION, $row['stripe_subscription_id']);
        $this->assertSame('trialing', $row['status']);
        $this->assertSame('solo', $row['plan']);
    }

    public function test_an_invoice_before_checkout_completes_applies_once_the_row_is_linked_by_customer(): void
    {
        $userId = $this->createUser('invoice-first@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);
        $this->remember($this->subscription('active', 'price_test_solo_monthly'));

        $paid = $this->event('evt_invoice_first', 'invoice.paid', [
            'object' => 'invoice',
            'customer' => self::CUSTOMER,
            'subscription' => self::SUBSCRIPTION,
        ]);
        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($paid, $this->signature($paid))));

        $afterInvoice = $this->subscriptionRow($userId);
        $this->assertSame(self::SUBSCRIPTION, $afterInvoice['stripe_subscription_id']);
        $this->assertSame('active', $afterInvoice['status']);

        $checkout = $this->event('evt_checkout_after_invoice', 'checkout.session.completed', $this->checkout($userId));
        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($checkout, $this->signature($checkout))));

        $afterCheckout = $this->subscriptionRow($userId);
        $this->assertSame(self::SUBSCRIPTION, $afterCheckout['stripe_subscription_id']);
        $this->assertSame('active', $afterCheckout['status']);
        $this->assertSame('solo', $afterCheckout['plan']);
    }

    public function test_a_checkout_for_a_deleted_user_cancels_the_subscription_and_writes_no_row(): void
    {
        $userId = $this->createUser('deleted-checkout@example.com');
        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$userId]);
        $this->remember($this->subscription('active', 'price_test_solo_monthly'));

        $payload = $this->event('evt_checkout_deleted_user', 'checkout.session.completed', $this->checkout($userId));

        $this->assertSame(['status' => 'ignored'], $this->body($this->deliver($payload, $this->signature($payload))));
        $this->assertSame([
            ['method' => 'get', 'path' => '/v1/subscriptions/' . self::SUBSCRIPTION],
            ['method' => 'delete', 'path' => '/v1/subscriptions/' . self::SUBSCRIPTION],
        ], $this->requests());

        $statement = $this->pdo()->prepare(
            'SELECT COUNT(*) FROM subscriptions WHERE stripe_subscription_id = ? OR stripe_customer_id = ?'
        );
        $statement->execute([self::SUBSCRIPTION, self::CUSTOMER]);
        $this->assertSame(0, (int) $statement->fetchColumn());
    }

    public function test_a_refused_cancel_for_a_deleted_user_rolls_back_the_event_and_answers_500(): void
    {
        $userId = $this->createUser('deleted-cancel-fails@example.com');
        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$userId]);
        $this->remember($this->subscription('active', 'price_test_solo_monthly'));
        $this->stripe->refuseCancellations();

        $payload = $this->event('evt_checkout_cancel_fails', 'checkout.session.completed', $this->checkout($userId));
        $response = $this->deliver($payload, $this->signature($payload));

        $this->assertError($response, 500, 'internal_error');

        $statement = $this->pdo()->prepare('SELECT COUNT(*) FROM stripe_events WHERE event_id = ?');
        $statement->execute(['evt_checkout_cancel_fails']);
        $this->assertSame(0, (int) $statement->fetchColumn());
    }

    public function test_a_checkout_for_a_deleted_user_leaves_an_already_ended_subscription_alone(): void
    {
        $userId = $this->createUser('deleted-checkout-ended@example.com');
        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$userId]);
        $this->remember($this->subscription('canceled', 'price_test_solo_monthly'));

        $payload = $this->event('evt_checkout_ended', 'checkout.session.completed', $this->checkout($userId));

        $this->assertSame(['status' => 'ignored'], $this->body($this->deliver($payload, $this->signature($payload))));
        $this->assertSame([
            ['method' => 'get', 'path' => '/v1/subscriptions/' . self::SUBSCRIPTION],
        ], $this->requests());
    }

    public function test_a_checkout_whose_customer_differs_from_the_row_cancels_a_live_subscription(): void
    {
        $userId = $this->createUser('checkout-other-customer@example.com');
        $this->pdo()->prepare(
            "INSERT INTO subscriptions (user_id, stripe_customer_id, status) VALUES (?, 'cus_other', 'none')"
        )->execute([$userId]);
        $this->remember($this->subscription('trialing', 'price_test_solo_monthly'));

        $payload = $this->event('evt_checkout_other_customer', 'checkout.session.completed', $this->checkout($userId));

        $this->assertSame(['status' => 'ignored'], $this->body($this->deliver($payload, $this->signature($payload))));
        $this->assertSame([
            ['method' => 'get', 'path' => '/v1/subscriptions/' . self::SUBSCRIPTION],
            ['method' => 'delete', 'path' => '/v1/subscriptions/' . self::SUBSCRIPTION],
        ], $this->requests());

        $statement = $this->pdo()->prepare(
            'SELECT stripe_customer_id, stripe_subscription_id FROM subscriptions WHERE user_id = ?'
        );
        $statement->execute([$userId]);
        $this->assertSame(
            ['stripe_customer_id' => 'cus_other', 'stripe_subscription_id' => null],
            $statement->fetch(\PDO::FETCH_ASSOC),
        );
    }

    public function test_an_invoice_for_an_unknown_customer_is_ignored_without_asking_stripe(): void
    {
        $paid = $this->event('evt_invoice_unknown', 'invoice.paid', [
            'object' => 'invoice',
            'customer' => 'cus_unknown',
            'subscription' => 'sub_unknown',
        ]);

        $this->assertSame(['status' => 'ignored'], $this->body($this->deliver($paid, $this->signature($paid))));
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_invoice_paid_activates_a_past_due_row_and_a_failed_payment_marks_a_paying_row_past_due(): void
    {
        $userId = $this->createUser('invoices@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'past_due',
        ]);

        $this->remember($this->subscription('active', 'price_test_solo_monthly'));
        $paid = $this->event('evt_invoice_paid', 'invoice.paid', ['object' => 'invoice', 'customer' => self::CUSTOMER, 'subscription' => self::SUBSCRIPTION]);
        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($paid, $this->signature($paid))));
        $this->assertSame('active', $this->subscriptionRow($userId)['status']);

        $this->remember($this->subscription('past_due', 'price_test_solo_monthly'));
        $failed = $this->event('evt_invoice_failed', 'invoice.payment_failed', ['object' => 'invoice', 'customer' => self::CUSTOMER, 'subscription' => self::SUBSCRIPTION]);
        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($failed, $this->signature($failed))));
        $this->assertSame('past_due', $this->subscriptionRow($userId)['status']);
    }

    public function test_a_late_failure_notice_cannot_grant_a_plan_back_to_a_canceled_subscription(): void
    {
        $userId = $this->createUser('late-failure@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'canceled',
        ]);

        $this->remember($this->subscription('canceled', 'price_test_solo_monthly'));
        $failed = $this->event('evt_late_failure', 'invoice.payment_failed', ['object' => 'invoice', 'customer' => self::CUSTOMER, 'subscription' => self::SUBSCRIPTION]);

        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($failed, $this->signature($failed))));
        $this->assertSame('canceled', $this->subscriptionRow($userId)['status']);
    }

    public function test_a_deleted_event_stores_the_status_that_stripe_returns(): void
    {
        $userId = $this->createUser('deleted-status@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'active',
        ]);

        $this->remember($this->subscription('incomplete_expired', 'price_test_solo_monthly'));
        $deleted = $this->event('evt_deleted_status', 'customer.subscription.deleted', $this->subscription('incomplete_expired', 'price_test_solo_monthly'));

        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($deleted, $this->signature($deleted))));
        $this->assertSame('incomplete_expired', $this->subscriptionRow($userId)['status']);
    }

    public function test_a_stale_updated_event_after_a_deleted_one_leaves_the_row_canceled(): void
    {
        $userId = $this->createUser('stale@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'active',
        ]);

        $this->remember($this->subscription('canceled', 'price_test_solo_monthly'));
        $deleted = $this->event('evt_deleted_first', 'customer.subscription.deleted', $this->subscription('canceled', 'price_test_solo_monthly'));
        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($deleted, $this->signature($deleted))));
        $this->assertSame('canceled', $this->subscriptionRow($userId)['status']);

        $stale = $this->event('evt_updated_late', 'customer.subscription.updated', $this->subscription('active', 'price_test_solo_monthly'));
        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($stale, $this->signature($stale))));

        $row = $this->subscriptionRow($userId);
        $this->assertSame('canceled', $row['status']);
        $this->assertSame(self::SUBSCRIPTION, $row['stripe_subscription_id']);
    }

    public function test_a_second_live_subscription_is_cancelled_and_the_row_keeps_the_first(): void
    {
        $userId = $this->createUser('duplicate@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'active',
        ]);

        $second = $this->subscription('active', 'price_test_friends_monthly', self::OTHER_SUBSCRIPTION);
        $this->remember($second);
        $payload = $this->event('evt_second_live', 'customer.subscription.created', $second);

        $this->assertSame(['status' => 'ignored'], $this->body($this->deliver($payload, $this->signature($payload))));

        $row = $this->subscriptionRow($userId);
        $this->assertSame(self::SUBSCRIPTION, $row['stripe_subscription_id']);
        $this->assertSame('active', $row['status']);
        $this->assertSame('solo', $row['plan']);
        $this->assertSame([
            ['method' => 'get', 'path' => '/v1/subscriptions/' . self::OTHER_SUBSCRIPTION],
            ['method' => 'delete', 'path' => '/v1/subscriptions/' . self::OTHER_SUBSCRIPTION],
        ], $this->requests());
    }

    public function test_a_second_subscription_that_is_not_live_leaves_the_row_and_is_not_cancelled(): void
    {
        $userId = $this->createUser('expired@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'active',
        ]);

        $second = $this->subscription('incomplete_expired', 'price_test_friends_monthly', self::OTHER_SUBSCRIPTION);
        $this->remember($second);
        $payload = $this->event('evt_second_expired', 'customer.subscription.created', $second);

        $this->assertSame(['status' => 'ignored'], $this->body($this->deliver($payload, $this->signature($payload))));
        $this->assertSame(self::SUBSCRIPTION, $this->subscriptionRow($userId)['stripe_subscription_id']);
        $this->assertSame([
            ['method' => 'get', 'path' => '/v1/subscriptions/' . self::OTHER_SUBSCRIPTION],
        ], $this->requests());
    }

    public function test_a_second_subscription_replaces_a_canceled_first(): void
    {
        $userId = $this->createUser('replace@example.com');
        $this->insertSubscription($userId, [
            'stripe_customer_id' => self::CUSTOMER,
            'stripe_subscription_id' => self::SUBSCRIPTION,
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'canceled',
        ]);

        $second = $this->subscription('active', 'price_test_friends_monthly', self::OTHER_SUBSCRIPTION);
        $this->remember($second);
        $payload = $this->event('evt_replacement', 'customer.subscription.created', $second);

        $this->assertSame(['status' => 'processed'], $this->body($this->deliver($payload, $this->signature($payload))));

        $row = $this->subscriptionRow($userId);
        $this->assertSame(self::OTHER_SUBSCRIPTION, $row['stripe_subscription_id']);
        $this->assertSame('active', $row['status']);
        $this->assertSame('friends', $row['plan']);
    }

    public function test_a_failed_retrieve_rolls_back_the_event_and_answers_500(): void
    {
        $userId = $this->createUser('retrieve-fails@example.com');
        $this->insertSubscription($userId, ['stripe_customer_id' => self::CUSTOMER]);

        $payload = $this->event('evt_retrieve_fails', 'customer.subscription.updated', $this->subscription('active', 'price_test_solo_monthly'));
        $response = $this->deliver($payload, $this->signature($payload));

        $this->assertError($response, 500, 'internal_error');

        $statement = $this->pdo()->prepare('SELECT COUNT(*) FROM stripe_events WHERE event_id = ?');
        $statement->execute(['evt_retrieve_fails']);
        $this->assertSame(0, (int) $statement->fetchColumn());
        $this->assertNull($this->subscriptionRow($userId)['stripe_subscription_id']);
    }

    public function test_a_webhook_without_billing_configuration_gives_503(): void
    {
        Config::set('STRIPE_WEBHOOK_SECRET', '');
        $payload = $this->event('evt_unconfigured', 'customer.created', ['id' => self::CUSTOMER, 'object' => 'customer']);

        $response = $this->deliver($payload, $this->signature($payload, secret: 'whsec_anything'));

        $this->assertError($response, 503, 'billing_unavailable');
    }

    /**
     * Configures the subscription the fake returns when the webhook retrieves it.
     *
     * @param array<string, mixed> $subscription
     * @return array<string, mixed>
     */
    private function remember(array $subscription): array
    {
        $this->stripe->setSubscription((string) $subscription['id'], $subscription);

        return $subscription;
    }

    /** @return array<string, mixed> */
    private function checkout(int $userId): array
    {
        return [
            'object' => 'checkout.session',
            'id' => 'cs_test_1',
            'client_reference_id' => (string) $userId,
            'customer' => self::CUSTOMER,
            'subscription' => self::SUBSCRIPTION,
        ];
    }

    /** @return list<array{method: string, path: string}> */
    private function requests(): array
    {
        return array_map(
            static fn (array $call): array => ['method' => $call['method'], 'path' => $call['path']],
            $this->stripe->calls(),
        );
    }

    /** @return array<string, mixed> */
    private function subscription(string $status, string $priceId, string $id = self::SUBSCRIPTION): array
    {
        return [
            'id' => $id,
            'object' => 'subscription',
            'customer' => self::CUSTOMER,
            'status' => $status,
            'trial_end' => 1900000000,
            'current_period_end' => 1900600000,
            'cancel_at_period_end' => false,
            'items' => [
                'object' => 'list',
                'data' => [['id' => 'si_test_1', 'price' => ['id' => $priceId]]],
            ],
        ];
    }

    /** @param array<string, mixed> $object */
    private function event(string $id, string $type, array $object): string
    {
        return json_encode([
            'id' => $id,
            'object' => 'event',
            'api_version' => '2025-01-27.acacia',
            'created' => time(),
            'livemode' => false,
            'pending_webhooks' => 1,
            'request' => ['id' => null, 'idempotency_key' => null],
            'type' => $type,
            'data' => ['object' => $object],
        ], JSON_THROW_ON_ERROR);
    }

    private function signature(string $payload, ?int $timestamp = null, ?string $secret = null): string
    {
        $time = $timestamp ?? time();
        $digest = hash_hmac('sha256', $time . '.' . $payload, $secret ?? self::WEBHOOK_SECRET);

        return 't=' . $time . ',v1=' . $digest;
    }

    private function deliver(string $payload, string $signature): \Psr\Http\Message\ResponseInterface
    {
        $headers = $signature === '' ? [] : ['Stripe-Signature' => $signature];

        return $this->send('POST', self::PATH, raw: $payload, headers: $headers);
    }
}
