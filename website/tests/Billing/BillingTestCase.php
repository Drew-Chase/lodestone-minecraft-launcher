<?php

declare(strict_types=1);

namespace tests\Billing;

use app\Auth\JwtIssuer;
use app\Config\Config;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * Sets the Stripe keys and prices, installs the fake Stripe client, and offers request and row helpers.
 */
abstract class BillingTestCase extends DatabaseTestCase
{
    protected const WEBHOOK_SECRET = 'whsec_test_secret';
    protected const IP = '198.51.100.60';

    /** @var array<string, string> */
    protected const PRICES = [
        'STRIPE_PRICE_SOLO_MONTHLY' => 'price_test_solo_monthly',
        'STRIPE_PRICE_SOLO_YEARLY' => 'price_test_solo_yearly',
        'STRIPE_PRICE_FRIENDS_MONTHLY' => 'price_test_friends_monthly',
        'STRIPE_PRICE_FRIENDS_YEARLY' => 'price_test_friends_yearly',
        'STRIPE_PRICE_UNLIMITED_MONTHLY' => 'price_test_unlimited_monthly',
        'STRIPE_PRICE_UNLIMITED_YEARLY' => 'price_test_unlimited_yearly',
    ];

    private static ?App $app = null;

    protected FakeStripeHttpClient $stripe;

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('STRIPE_SECRET_KEY', 'sk_test_fake');
        Config::set('STRIPE_WEBHOOK_SECRET', self::WEBHOOK_SECRET);
        foreach (self::PRICES as $key => $value) {
            Config::set($key, $value);
        }

        $this->stripe = new FakeStripeHttpClient();
        $this->stripe->install();
    }

    protected function tearDown(): void
    {
        FakeStripeHttpClient::uninstall();
        Config::reset();

        parent::tearDown();
    }

    protected function app(): App
    {
        if (self::$app === null) {
            self::$app = require dirname(__DIR__, 2) . '/api/bootstrap.php';
        }

        return self::$app;
    }

    /**
     * @param array<string, mixed>|null $json
     * @param array<string, string> $headers
     */
    protected function send(
        string $method,
        string $path,
        ?array $json = null,
        ?string $raw = null,
        array $headers = [],
    ): ResponseInterface {
        $request = (new ServerRequestFactory())->createServerRequest($method, $path, ['REMOTE_ADDR' => self::IP]);

        $payload = $raw ?? ($json === null ? null : json_encode($json, JSON_THROW_ON_ERROR));
        if ($payload !== null) {
            $request = $request->withBody((new StreamFactory())->createStream($payload));
        }

        foreach ($headers as $name => $value) {
            $request = $request->withHeader($name, $value);
        }

        return $this->app()->handle($request);
    }

    /** @return array<string, string> */
    protected function bearer(int $userId): array
    {
        return ['Authorization' => 'Bearer ' . JwtIssuer::fromConfig()->issue($userId, 'family_test_1')['token']];
    }

    protected function createUser(string $email, bool $verified = true): int
    {
        $statement = $this->pdo()->prepare('INSERT INTO users (email, email_verified_at) VALUES (?, ?)');
        $statement->execute([$email, $verified ? gmdate('Y-m-d H:i:s') : null]);

        return (int) $this->pdo()->lastInsertId();
    }

    /** @param array<string, mixed> $values Column name => value, for a subscriptions row. */
    protected function insertSubscription(int $userId, array $values): void
    {
        $columns = array_keys($values);
        $placeholders = implode(', ', array_fill(0, count($columns), '?'));

        $statement = $this->pdo()->prepare(
            'INSERT INTO subscriptions (user_id, ' . implode(', ', $columns) . ") VALUES (?, $placeholders)"
        );
        $statement->execute([$userId, ...array_values($values)]);
    }

    /** @return array<string, mixed> */
    protected function subscriptionRow(int $userId): array
    {
        $statement = $this->pdo()->prepare('SELECT * FROM subscriptions WHERE user_id = ?');
        $statement->execute([$userId]);
        $row = $statement->fetch();
        $this->assertIsArray($row);

        return $row;
    }

    /** @return array<string, mixed> */
    protected function body(ResponseInterface $response): array
    {
        $decoded = json_decode((string) $response->getBody(), true, 512, JSON_THROW_ON_ERROR);
        $this->assertIsArray($decoded);

        return $decoded;
    }

    protected function assertError(ResponseInterface $response, int $status, string $code): void
    {
        $this->assertSame($status, $response->getStatusCode());
        $this->assertSame(['error' => $code], $this->body($response));
    }
}
