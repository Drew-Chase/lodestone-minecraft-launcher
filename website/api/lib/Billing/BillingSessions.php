<?php

declare(strict_types=1);

namespace app\Billing;

use app\Auth\AuthException;
use app\Auth\RateLimiter;
use app\Users\UserRepository;
use Stripe\Exception\ExceptionInterface;
use Stripe\StripeClient;

/**
 * Starts a Stripe Checkout subscription and a customer portal session for a user.
 */
final class BillingSessions
{
    private const TRIAL_DAYS = 7;

    private const CHECKOUT_LIMIT = 10;

    private const CHECKOUT_WINDOW_SECONDS = 3600;

    /** Statuses in which the user already has a subscription to manage in the portal. */
    private const MANAGED = ['trialing', 'active', 'past_due', 'incomplete'];

    public function __construct(
        private readonly SubscriptionStore $store,
        private readonly UserRepository $users,
        private readonly StripeClient $stripe,
        private readonly string $appUrl,
    ) {
    }

    /**
     * @throws AuthException unauthorized (401), email_not_verified (403), billing_unavailable (503),
     *     already_subscribed (409), rate_limited (429) or billing_failed (502)
     */
    public function checkout(int $userId, string $plan, string $interval, PriceMap $prices): string
    {
        $user = $this->users->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            throw new AuthException('unauthorized', 401);
        }

        if ($user['email_verified_at'] === null) {
            throw new AuthException('email_not_verified', 403);
        }

        $priceId = $prices->priceId($plan, $interval);
        if ($priceId === null) {
            throw new AuthException('billing_unavailable', 503);
        }

        $row = $this->store->find($userId);
        if ($row !== null && in_array($row['status'], self::MANAGED, true)) {
            throw new AuthException('already_subscribed', 409);
        }

        $limiter = new RateLimiter();
        $bucket = 'billing:checkout:' . $userId;
        if ($limiter->hit($bucket, self::CHECKOUT_WINDOW_SECONDS) > self::CHECKOUT_LIMIT) {
            throw new AuthException('rate_limited', 429, $limiter->retryAfter(self::CHECKOUT_WINDOW_SECONDS));
        }

        $customerId = $this->customerFor($userId, (string) $user['email'], $row);

        $session = $this->call(fn () => $this->stripe->checkout->sessions->create([
            'mode' => 'subscription',
            'line_items' => [['price' => $priceId, 'quantity' => 1]],
            'subscription_data' => ['trial_period_days' => self::TRIAL_DAYS],
            'payment_method_collection' => 'always',
            'customer' => $customerId,
            'client_reference_id' => (string) $userId,
            'success_url' => $this->appUrl . '/dashboard/billing?checkout=success',
            'cancel_url' => $this->appUrl . '/subscribe?checkout=cancelled',
        ]));

        return $this->urlOf($session->url);
    }

    /**
     * @throws AuthException no_billing_account (404) or billing_failed (502)
     */
    public function portal(int $userId): string
    {
        $row = $this->store->find($userId);
        $customerId = $row['stripe_customer_id'] ?? null;
        if (!is_string($customerId) || $customerId === '') {
            throw new AuthException('no_billing_account', 404);
        }

        $session = $this->call(fn () => $this->stripe->billingPortal->sessions->create([
            'customer' => $customerId,
            'return_url' => $this->appUrl . '/dashboard/billing',
        ]));

        return $this->urlOf($session->url);
    }

    /** @param array<string, mixed>|null $row */
    private function customerFor(int $userId, string $email, ?array $row): string
    {
        $existing = $row['stripe_customer_id'] ?? null;
        if (is_string($existing) && $existing !== '') {
            return $existing;
        }

        $customer = $this->call(fn () => $this->stripe->customers->create([
            'email' => $email,
            'metadata' => ['user_id' => (string) $userId],
        ]));

        return $this->store->attachCustomer($userId, (string) $customer->id);
    }

    /** Runs a Stripe request. Any SDK failure becomes billing_failed, and only its class is logged. */
    private function call(callable $request): mixed
    {
        try {
            return $request();
        } catch (ExceptionInterface $error) {
            error_log('Stripe request failed: ' . $error::class);

            throw new AuthException('billing_failed', 502);
        }
    }

    private function urlOf(?string $url): string
    {
        if ($url === null || $url === '') {
            throw new AuthException('billing_failed', 502);
        }

        return $url;
    }
}
