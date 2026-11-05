<?php

declare(strict_types=1);

namespace app\Billing;

use app\Auth\AuthException;
use app\Db\Database;
use Stripe\Event;
use Stripe\Exception\SignatureVerificationException;
use Stripe\Exception\UnexpectedValueException;
use Stripe\StripeClient;
use Stripe\Webhook;

/**
 * Verifies a Stripe webhook and applies its event. Each event runs in one transaction with its record in
 * stripe_events, so a failure rolls the record back and Stripe's retry processes the event again.
 *
 * A subscription's state is always read from Stripe, never from the event payload, because Stripe does not
 * guarantee the order in which events arrive.
 */
final class WebhookHandler
{
    /** Stripe's own replay window, in seconds. */
    private const TOLERANCE = 300;

    public function __construct(
        private readonly SubscriptionStore $store,
        private readonly PriceMap $prices,
        private readonly StripeClient $stripe,
    ) {
    }

    /** @throws AuthException invalid_signature (400) when the header or the signature does not verify */
    public static function verify(string $payload, string $signatureHeader, string $secret): Event
    {
        if ($signatureHeader === '') {
            throw new AuthException('invalid_signature', 400);
        }

        try {
            return Webhook::constructEvent($payload, $signatureHeader, $secret, self::TOLERANCE);
        } catch (SignatureVerificationException | UnexpectedValueException) {
            throw new AuthException('invalid_signature', 400);
        }
    }

    /**
     * @return string processed, ignored or duplicate
     * @throws \Throwable when Stripe cannot be reached for a subscription; the event is then recorded nowhere
     */
    public function process(Event $event): string
    {
        $type = (string) $event->type;
        $eventId = (string) $event->id;

        return Database::transaction(function () use ($event, $type, $eventId): string {
            if (!$this->store->recordEvent($eventId, $type)) {
                return 'duplicate';
            }

            $object = $event->toArray()['data']['object'] ?? [];

            return $this->apply($type, is_array($object) ? $object : []) ? 'processed' : 'ignored';
        });
    }

    /** @param array<string, mixed> $object */
    private function apply(string $type, array $object): bool
    {
        return match ($type) {
            'checkout.session.completed' => $this->checkoutCompleted($object),
            'customer.subscription.created',
            'customer.subscription.updated',
            'customer.subscription.deleted' => $this->subscriptionChanged($object),
            'invoice.paid', 'invoice.payment_failed' => $this->invoiceChanged($object),
            default => false,
        };
    }

    /** @param array<string, mixed> $session */
    private function checkoutCompleted(array $session): bool
    {
        $userId = $session['client_reference_id'] ?? null;
        $customerId = $session['customer'] ?? null;
        $subscriptionId = $session['subscription'] ?? null;

        if (
            !is_string($userId) || !ctype_digit($userId)
            || !is_string($customerId) || !is_string($subscriptionId)
        ) {
            return false;
        }

        $id = (int) $userId;
        if (!$this->store->userExists($id)) {
            $this->cancelIfLive($subscriptionId);

            return false;
        }

        // A row that already holds another customer cannot record this subscription.
        if ($this->store->attachCustomer($id, $customerId) !== $customerId) {
            $this->cancelIfLive($subscriptionId);

            return false;
        }

        return $this->applySubscription($id, $subscriptionId);
    }

    /**
     * Cancels a subscription that no row can hold, unless Stripe already reports it as ended. A refused call throws,
     * which rolls the transaction back and leaves the event for Stripe to retry.
     */
    private function cancelIfLive(string $subscriptionId): void
    {
        $status = (string) $this->stripe->subscriptions->retrieve($subscriptionId)->status;
        if (SubscriptionStore::isLive($status)) {
            $this->stripe->subscriptions->cancel($subscriptionId);
        }
    }

    /** @param array<string, mixed> $subscription */
    private function subscriptionChanged(array $subscription): bool
    {
        $customerId = $subscription['customer'] ?? null;
        $subscriptionId = $subscription['id'] ?? null;
        if (!is_string($customerId) || !is_string($subscriptionId)) {
            return false;
        }

        $row = $this->store->findByCustomer($customerId);
        if ($row === null) {
            return false;
        }

        return $this->applySubscription((int) $row['user_id'], $subscriptionId);
    }

    /** @param array<string, mixed> $invoice */
    private function invoiceChanged(array $invoice): bool
    {
        $customerId = $invoice['customer'] ?? null;
        $subscriptionId = $invoice['subscription'] ?? $invoice['parent']['subscription_details']['subscription'] ?? null;
        if (!is_string($customerId) || !is_string($subscriptionId)) {
            return false;
        }

        $row = $this->store->findByCustomer($customerId);
        if ($row === null) {
            return false;
        }

        return $this->applySubscription((int) $row['user_id'], $subscriptionId);
    }

    /**
     * Retrieves the subscription and applies it to the user's row under the row lock. The row keeps its current
     * subscription while that one is live, and a second live subscription is cancelled rather than stored.
     *
     * @return bool true when the row now holds the subscription, false when the row was left unchanged
     */
    private function applySubscription(int $userId, string $subscriptionId): bool
    {
        $row = $this->store->lockForUpdate($userId);
        if ($row === null) {
            return false;
        }

        $subscription = $this->stripe->subscriptions->retrieve($subscriptionId)->toArray();
        $status = is_string($subscription['status'] ?? null) ? $subscription['status'] : 'none';

        $held = $row['stripe_subscription_id'];
        if (is_string($held) && $held !== $subscriptionId && SubscriptionStore::isLive((string) $row['status'])) {
            if (SubscriptionStore::isLive($status)) {
                $this->stripe->subscriptions->cancel($subscriptionId);
            }

            return false;
        }

        $this->store->saveSubscription($userId, SubscriptionSync::columns($subscription, $status, $this->prices));

        return true;
    }
}
