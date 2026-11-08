<?php

declare(strict_types=1);

namespace tests\Http;

use app\Config\Config;
use app\Http\ClientIp;
use PHPUnit\Framework\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

final class ClientIpTest extends TestCase
{
    protected function tearDown(): void
    {
        Config::reset();
    }

    public function test_the_connection_address_is_used_when_no_proxy_is_trusted(): void
    {
        Config::set('TRUSTED_PROXIES', '');

        self::assertSame('198.51.100.9', ClientIp::from($this->request('198.51.100.9', '203.0.113.5')));
    }

    public function test_a_forwarded_header_from_an_untrusted_peer_is_ignored(): void
    {
        Config::set('TRUSTED_PROXIES', '10.0.0.1');

        self::assertSame('198.51.100.9', ClientIp::from($this->request('198.51.100.9', '203.0.113.5')));
    }

    public function test_behind_a_trusted_proxy_the_client_is_the_right_most_untrusted_address(): void
    {
        Config::set('TRUSTED_PROXIES', '10.0.0.0/8');

        self::assertSame('203.0.113.5', ClientIp::from($this->request('10.1.2.3', '203.0.113.5, 10.9.9.9')));
    }

    public function test_a_malformed_forwarded_entry_falls_back_to_the_proxy(): void
    {
        Config::set('TRUSTED_PROXIES', '10.0.0.0/8');

        self::assertSame('10.1.2.3', ClientIp::from($this->request('10.1.2.3', 'not-an-address')));
    }

    private function request(string $peer, string $forwardedFor): \Psr\Http\Message\ServerRequestInterface
    {
        return (new ServerRequestFactory())
            ->createServerRequest('GET', '/', ['REMOTE_ADDR' => $peer])
            ->withHeader('X-Forwarded-For', $forwardedFor);
    }
}
