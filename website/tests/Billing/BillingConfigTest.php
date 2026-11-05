<?php

declare(strict_types=1);

namespace tests\Billing;

use app\Billing\BillingConfig;
use Stripe\ApiRequestor;
use Stripe\HttpClient\CurlClient;

final class BillingConfigTest extends BillingTestCase
{
    public function test_the_client_sets_a_10_second_request_timeout_and_a_5_second_connect_timeout(): void
    {
        FakeStripeHttpClient::uninstall();

        BillingConfig::client();

        $curl = ApiRequestor::httpClient();
        $this->assertInstanceOf(CurlClient::class, $curl);
        $this->assertSame(10, $curl->getTimeout());
        $this->assertSame(5, $curl->getConnectTimeout());
    }

    public function test_an_installed_fake_stays_the_request_client_after_the_client_is_built(): void
    {
        BillingConfig::client();

        $this->assertSame($this->stripe, ApiRequestor::httpClient());
    }
}
