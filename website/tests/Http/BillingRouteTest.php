<?php

declare(strict_types=1);

namespace tests\Http;

use app\Config\Config;
use tests\Billing\BillingTestCase;

/**
 * HTTP behaviour of the /api/billing routes, with the fake Stripe client installed by the base class.
 */
final class BillingRouteTest extends BillingTestCase
{
    private const EMAIL = 'buyer@example.com';

    public function test_plans_is_public_and_lists_the_three_paid_plans(): void
    {
        $response = $this->send('GET', '/api/billing/plans');

        $this->assertSame(200, $response->getStatusCode());
        $plans = $this->body($response)['plans'];

        $this->assertSame(['solo', 'friends', 'unlimited'], array_column($plans, 'id'));
        $this->assertSame(
            [
                'id' => 'unlimited',
                'name' => 'Unlimited',
                'storage_bytes' => 500_000_000_000,
                'devices' => null,
                'shares' => null,
                'intervals' => ['monthly', 'yearly'],
            ],
            $plans[2],
        );
    }

    public function test_checkout_by_an_unverified_user_gives_403(): void
    {
        $userId = $this->createUser('unverified@example.com', verified: false);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly'], headers: $this->bearer($userId));

        $this->assertError($response, 403, 'email_not_verified');
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_checkout_without_billing_configuration_gives_503(): void
    {
        Config::set('STRIPE_SECRET_KEY', '');
        $userId = $this->createUser(self::EMAIL);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly'], headers: $this->bearer($userId));

        $this->assertError($response, 503, 'billing_unavailable');
    }

    public function test_checkout_with_a_missing_price_gives_503(): void
    {
        Config::set('STRIPE_PRICE_FRIENDS_YEARLY', '');
        $userId = $this->createUser(self::EMAIL);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly'], headers: $this->bearer($userId));

        $this->assertError($response, 503, 'billing_unavailable');
    }

    public function test_checkout_without_a_plan_gives_missing_field_and_an_unknown_plan_gives_invalid_plan(): void
    {
        $userId = $this->createUser(self::EMAIL);
        $headers = $this->bearer($userId);

        $this->assertError(
            $this->send('POST', '/api/billing/checkout', ['interval' => 'monthly'], headers: $headers),
            400,
            'missing_field',
        );
        $this->assertError(
            $this->send('POST', '/api/billing/checkout', ['plan' => 7, 'interval' => 'monthly'], headers: $headers),
            400,
            'missing_field',
        );
        $this->assertError(
            $this->send('POST', '/api/billing/checkout', ['plan' => 'gold', 'interval' => 'monthly'], headers: $headers),
            400,
            'invalid_plan',
        );
        $this->assertError(
            $this->send('POST', '/api/billing/checkout', ['plan' => 'free', 'interval' => 'monthly'], headers: $headers),
            400,
            'invalid_plan',
        );
        $this->assertError(
            $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'weekly'], headers: $headers),
            400,
            'invalid_plan',
        );
    }

    public function test_checkout_without_a_bearer_token_gives_401(): void
    {
        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly']);

        $this->assertError($response, 401, 'unauthorized');
    }

    public function test_a_successful_checkout_returns_the_session_url_and_stores_the_customer(): void
    {
        $userId = $this->createUser(self::EMAIL);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'friends', 'interval' => 'yearly'], headers: $this->bearer($userId));

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['url' => 'https://checkout.stripe.test/c/pay/cs_test_1'], $this->body($response));
        $this->assertSame('cus_test_1', $this->subscriptionRow($userId)['stripe_customer_id']);

        $calls = $this->stripe->calls();
        $this->assertSame(['/v1/customers', '/v1/checkout/sessions'], array_column($calls, 'path'));

        $this->assertSame(self::EMAIL, $calls[0]['params']['email']);
        $this->assertSame(['user_id' => (string) $userId], $calls[0]['params']['metadata']);

        $session = $calls[1]['params'];
        $this->assertSame('subscription', $session['mode']);
        $this->assertSame([['price' => 'price_test_friends_yearly', 'quantity' => 1]], $session['line_items']);
        $this->assertSame(['trial_period_days' => 7], $session['subscription_data']);
        $this->assertSame('always', $session['payment_method_collection']);
        $this->assertSame('cus_test_1', $session['customer']);
        $this->assertSame((string) $userId, $session['client_reference_id']);
        $this->assertSame(rtrim(Config::need('APP_URL'), '/') . '/dashboard/billing?checkout=success', $session['success_url']);
        $this->assertSame(rtrim(Config::need('APP_URL'), '/') . '/subscribe?checkout=cancelled', $session['cancel_url']);
    }

    public function test_a_second_checkout_while_trialing_gives_409_and_creates_nothing(): void
    {
        $userId = $this->createUser(self::EMAIL);
        $this->insertSubscription($userId, [
            'stripe_customer_id' => 'cus_test_1',
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'trialing',
        ]);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly'], headers: $this->bearer($userId));

        $this->assertError($response, 409, 'already_subscribed');
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_a_subscribed_user_with_a_missing_price_gives_503_rather_than_409(): void
    {
        Config::set('STRIPE_PRICE_FRIENDS_YEARLY', '');
        $userId = $this->createUser(self::EMAIL);
        $this->insertSubscription($userId, [
            'stripe_customer_id' => 'cus_test_1',
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'trialing',
        ]);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'friends', 'interval' => 'yearly'], headers: $this->bearer($userId));

        $this->assertError($response, 503, 'billing_unavailable');
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_a_second_checkout_while_incomplete_gives_409_and_creates_nothing(): void
    {
        $userId = $this->createUser(self::EMAIL);
        $this->insertSubscription($userId, [
            'stripe_customer_id' => 'cus_test_1',
            'stripe_subscription_id' => 'sub_test_1',
            'plan' => 'solo',
            'billing_interval' => 'monthly',
            'status' => 'incomplete',
        ]);

        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'friends', 'interval' => 'monthly'], headers: $this->bearer($userId));

        $this->assertError($response, 409, 'already_subscribed');
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_the_eleventh_checkout_start_within_an_hour_gets_429_with_retry_after(): void
    {
        $userId = $this->createUser(self::EMAIL);
        $headers = $this->bearer($userId);

        for ($start = 1; $start <= 10; $start++) {
            $this->assertSame(
                200,
                $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly'], headers: $headers)->getStatusCode(),
            );
        }

        $callsBefore = count($this->stripe->calls());
        $response = $this->send('POST', '/api/billing/checkout', ['plan' => 'solo', 'interval' => 'monthly'], headers: $headers);

        $this->assertError($response, 429, 'rate_limited');
        $this->assertGreaterThanOrEqual(1, (int) $response->getHeaderLine('Retry-After'));
        $this->assertLessThanOrEqual(3600, (int) $response->getHeaderLine('Retry-After'));
        $this->assertCount($callsBefore, $this->stripe->calls());
    }

    public function test_the_portal_without_a_customer_gives_404(): void
    {
        $userId = $this->createUser(self::EMAIL);

        $response = $this->send('POST', '/api/billing/portal', headers: $this->bearer($userId));

        $this->assertError($response, 404, 'no_billing_account');
        $this->assertSame([], $this->stripe->calls());
    }

    public function test_the_portal_returns_a_session_url_for_the_customer(): void
    {
        $userId = $this->createUser(self::EMAIL);
        $this->insertSubscription($userId, ['stripe_customer_id' => 'cus_test_1', 'status' => 'canceled']);

        $response = $this->send('POST', '/api/billing/portal', headers: $this->bearer($userId));

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['url' => 'https://billing.stripe.test/p/session/bps_test_1'], $this->body($response));

        $calls = $this->stripe->calls();
        $this->assertSame('/v1/billing_portal/sessions', $calls[0]['path']);
        $this->assertSame('cus_test_1', $calls[0]['params']['customer']);
        $this->assertSame(rtrim(Config::need('APP_URL'), '/') . '/dashboard/billing', $calls[0]['params']['return_url']);
    }

    public function test_the_portal_without_billing_configuration_gives_503(): void
    {
        Config::set('STRIPE_SECRET_KEY', '');
        $userId = $this->createUser(self::EMAIL);

        $response = $this->send('POST', '/api/billing/portal', headers: $this->bearer($userId));

        $this->assertError($response, 503, 'billing_unavailable');
    }

    public function test_the_subscription_returns_the_free_plan_by_default(): void
    {
        $userId = $this->createUser(self::EMAIL);

        $response = $this->send('GET', '/api/billing/subscription', headers: $this->bearer($userId));

        $this->assertSame(200, $response->getStatusCode());
        $subscription = $this->body($response)['subscription'];
        $this->assertSame('free', $subscription['plan']);
        $this->assertSame('none', $subscription['status']);
        $this->assertSame(0, $subscription['storage_bytes']);
    }
}
