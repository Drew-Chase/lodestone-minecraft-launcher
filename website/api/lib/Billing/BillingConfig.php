<?php

declare(strict_types=1);

namespace app\Billing;

use app\Auth\AuthException;
use app\Config\Config;
use Stripe\ApiRequestor;
use Stripe\HttpClient\CurlClient;
use Stripe\StripeClient;

/**
 * The Stripe settings each billing route needs. A missing or empty key answers 503 billing_unavailable.
 */
final class BillingConfig
{
    /** Seconds allowed for a Stripe request to complete. Requests run while a user's subscription row is locked. */
    private const REQUEST_TIMEOUT = 10;

    /** Seconds allowed to open the connection to Stripe. */
    private const CONNECT_TIMEOUT = 5;

    /** The cURL client this class configured, so that it is not configured again. */
    private static ?CurlClient $timed = null;

    private function __construct()
    {
    }

    /** @throws AuthException billing_unavailable (503) when the secret key is not set */
    public static function client(): StripeClient
    {
        $key = Config::get('STRIPE_SECRET_KEY');
        if ($key === null) {
            throw new AuthException('billing_unavailable', 503);
        }

        self::limitTimeouts();

        return new StripeClient(['api_key' => $key]);
    }

    /**
     * The SDK's StripeClient takes no HTTP client option, and every request goes through the static client in
     * ApiRequestor. The timeouts are set on that client. A client installed by anything else, such as a test fake,
     * is left as it is.
     */
    private static function limitTimeouts(): void
    {
        $current = ApiRequestor::httpClient();
        if (!$current instanceof CurlClient || $current === self::$timed) {
            return;
        }

        self::$timed = (new CurlClient())
            ->setTimeout(self::REQUEST_TIMEOUT)
            ->setConnectTimeout(self::CONNECT_TIMEOUT);
        ApiRequestor::setHttpClient(self::$timed);
    }

    /** @throws AuthException billing_unavailable (503) when the webhook secret is not set */
    public static function webhookSecret(): string
    {
        $secret = Config::get('STRIPE_WEBHOOK_SECRET');
        if ($secret === null) {
            throw new AuthException('billing_unavailable', 503);
        }

        return $secret;
    }

    /** @throws AuthException billing_unavailable (503) when any of the six price keys is not set */
    public static function prices(): PriceMap
    {
        return PriceMap::fromConfig() ?? throw new AuthException('billing_unavailable', 503);
    }

    public static function appUrl(): string
    {
        return rtrim(Config::need('APP_URL'), '/');
    }
}
