<?php

declare(strict_types=1);

namespace tests\Billing;

use Stripe\ApiRequestor;
use Stripe\HttpClient\ClientInterface;

/**
 * Answers the Stripe requests the billing tests make, without the network. install() routes every StripeClient
 * request through this object, and uninstall() puts the default cURL client back. A subscription answers a GET
 * with the object set by setSubscription(), and a DELETE cancels it, as Stripe does.
 */
final class FakeStripeHttpClient implements ClientInterface
{
    /** @var list<array{method: string, path: string, params: array<string, mixed>}> */
    private array $calls = [];

    /** @var array<string, array<string, mixed>> Subscription objects by id, as Stripe would return them. */
    private array $subscriptions = [];

    private bool $refuseCancellations = false;

    /** Makes every cancel fail, as Stripe does for a subscription it will not cancel. */
    public function refuseCancellations(): void
    {
        $this->refuseCancellations = true;
    }

    public function install(): void
    {
        ApiRequestor::setHttpClient($this);
    }

    public static function uninstall(): void
    {
        ApiRequestor::setHttpClient(null);
    }

    /** @param array<string, mixed> $subscription The subscription object a retrieve of its id returns. */
    public function setSubscription(string $id, array $subscription): void
    {
        $this->subscriptions[$id] = $subscription;
    }

    /** @return list<array{method: string, path: string, params: array<string, mixed>}> */
    public function calls(): array
    {
        return $this->calls;
    }

    public function request($method, $absUrl, $headers, $params, $hasFile, $apiMode = 'v1', $maxNetworkRetries = null)
    {
        $path = (string) parse_url($absUrl, PHP_URL_PATH);
        $this->calls[] = [
            'method' => (string) $method,
            'path' => $path,
            'params' => is_array($params) ? $params : [],
        ];

        $reply = $this->reply((string) $method, $path);

        $responseHeaders = ['request-id' => 'req_fake_1', 'content-type' => 'application/json'];

        if ($reply === null) {
            $error = ['error' => ['type' => 'invalid_request_error', 'message' => 'Not answered by the fake.']];

            return [json_encode($error, JSON_THROW_ON_ERROR), 404, $responseHeaders];
        }

        return [json_encode($reply, JSON_THROW_ON_ERROR), 200, $responseHeaders];
    }

    /** @return array<string, mixed>|null */
    private function reply(string $method, string $path): ?array
    {
        if (preg_match('#^/v1/subscriptions/([^/]+)$#', $path, $match) === 1 && isset($this->subscriptions[$match[1]])) {
            return match ($method) {
                'get' => $this->subscriptions[$match[1]],
                'delete' => $this->refuseCancellations ? null : $this->cancel($match[1]),
                default => null,
            };
        }

        return match ($method . ' ' . $path) {
            'post /v1/customers' => ['object' => 'customer', 'id' => 'cus_test_1'],
            'post /v1/checkout/sessions' => [
                'object' => 'checkout.session',
                'id' => 'cs_test_1',
                'url' => 'https://checkout.stripe.test/c/pay/cs_test_1',
            ],
            'post /v1/billing_portal/sessions' => [
                'object' => 'billing_portal.session',
                'id' => 'bps_test_1',
                'url' => 'https://billing.stripe.test/p/session/bps_test_1',
            ],
            default => null,
        };
    }

    /** @return array<string, mixed> The subscription, canceled, as the DELETE returns it. */
    private function cancel(string $id): array
    {
        $this->subscriptions[$id]['status'] = 'canceled';

        return $this->subscriptions[$id];
    }
}
