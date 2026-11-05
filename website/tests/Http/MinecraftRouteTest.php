<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\JwtIssuer;
use app\Minecraft\MinecraftAccounts;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Minecraft\LinkFixtures;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of the /api/minecraft routes. Microsoft, Xbox and Minecraft are answered by the fake transport.
 */
final class MinecraftRouteTest extends DatabaseTestCase
{
    use LinkFixtures;

    private const IP = '198.51.100.70';
    private const ISO_PATTERN = '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00$/';

    private static ?App $app = null;

    protected function setUp(): void
    {
        parent::setUp();

        $this->installTransport();
    }

    protected function tearDown(): void
    {
        $this->restoreTransport();

        parent::tearDown();
    }

    public function testStartReturnsTheCodeToEnterAndNoDeviceCode(): void
    {
        $userId = $this->createUser('steve@example.test');
        $this->answerDeviceCode();

        $response = $this->send('POST', '/api/minecraft/link', token: $this->tokenFor($userId));

        self::assertSame(201, $response->getStatusCode());
        $body = $this->body($response);
        self::assertSame(['id', 'user_code', 'verification_uri', 'expires_at', 'interval'], array_keys($body));
        self::assertSame(32, strlen((string) $body['id']));
        self::assertSame(self::USER_CODE, $body['user_code']);
        self::assertSame('https://www.microsoft.com/link', $body['verification_uri']);
        self::assertSame(5, $body['interval']);
        self::assertMatchesRegularExpression(self::ISO_PATTERN, (string) $body['expires_at']);
    }

    public function testStatusOfAPendingSignInReportsPending(): void
    {
        $userId = $this->createUser('steve@example.test');
        $token = $this->tokenFor($userId);
        $this->answerDeviceCode();
        $id = $this->body($this->send('POST', '/api/minecraft/link', token: $token))['id'];
        $this->answerPoll(['error' => 'authorization_pending']);

        $response = $this->send('GET', '/api/minecraft/link/' . $id, token: $token);

        self::assertSame(200, $response->getStatusCode());
        self::assertSame(['status' => 'pending', 'interval' => 5], $this->body($response));
    }

    public function testStatusOfAMissingSessionIsNotFoundAndNotSlowDown(): void
    {
        $userId = $this->createUser('steve@example.test');

        $response = $this->send('GET', '/api/minecraft/link/' . bin2hex(random_bytes(16)), token: $this->tokenFor($userId));

        $this->assertError($response, 404, 'not_found');
    }

    public function testTheEleventhStartWithinAnHourIsRateLimited(): void
    {
        $userId = $this->createUser('steve@example.test');
        $token = $this->tokenFor($userId);
        $this->answerDeviceCode();

        for ($start = 1; $start <= 10; $start++) {
            self::assertSame(201, $this->send('POST', '/api/minecraft/link', token: $token)->getStatusCode());
        }

        $response = $this->send('POST', '/api/minecraft/link', token: $token);

        $this->assertError($response, 429, 'rate_limited');
        $retryAfter = (int) $response->getHeaderLine('Retry-After');
        self::assertGreaterThanOrEqual(1, $retryAfter);
        self::assertLessThanOrEqual(3600, $retryAfter);
    }

    public function testAnotherUsersSessionIsNotFound(): void
    {
        $owner = $this->createUser('steve@example.test');
        $stranger = $this->createUser('alex@example.test');
        $this->answerDeviceCode();
        $id = $this->body($this->send('POST', '/api/minecraft/link', token: $this->tokenFor($owner)))['id'];

        $response = $this->send('GET', '/api/minecraft/link/' . $id, token: $this->tokenFor($stranger));

        $this->assertError($response, 404, 'not_found');
    }

    public function testTheLinkedAccountsAreListed(): void
    {
        $userId = $this->createUser('steve@example.test');
        (new MinecraftAccounts())->link($userId, self::UUID, self::NAME);

        $response = $this->send('GET', '/api/minecraft/accounts', token: $this->tokenFor($userId));

        self::assertSame(200, $response->getStatusCode());
        $accounts = $this->body($response)['accounts'];
        self::assertCount(1, $accounts);
        self::assertSame(self::UUID, $accounts[0]['uuid']);
        self::assertSame(self::NAME, $accounts[0]['name']);
        self::assertMatchesRegularExpression(self::ISO_PATTERN, (string) $accounts[0]['linked_at']);
    }

    public function testUnlinkRemovesTheAccountOnlyForItsOwner(): void
    {
        $owner = $this->createUser('steve@example.test');
        $stranger = $this->createUser('alex@example.test');
        (new MinecraftAccounts())->link($owner, self::UUID, self::NAME);
        $path = '/api/minecraft/accounts/' . self::UUID;

        $this->assertError($this->send('DELETE', $path, token: $this->tokenFor($stranger)), 404, 'not_found');
        self::assertSame(204, $this->send('DELETE', $path, token: $this->tokenFor($owner))->getStatusCode());

        $remaining = $this->body($this->send('GET', '/api/minecraft/accounts', token: $this->tokenFor($owner)));
        self::assertSame(['accounts' => []], $remaining);
        $this->assertError($this->send('DELETE', $path, token: $this->tokenFor($owner)), 404, 'not_found');
    }

    public function testTheRoutesNeedABearerToken(): void
    {
        $this->assertError($this->send('POST', '/api/minecraft/link'), 401, 'unauthorized');
        $this->assertError($this->send('GET', '/api/minecraft/accounts'), 401, 'unauthorized');
    }

    public function testADeletedAccountIsUnauthorizedOnEveryRouteBeforeAnyWork(): void
    {
        $userId = $this->createUser('steve@example.test');
        $token = $this->tokenFor($userId);
        $this->answerDeviceCode();
        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$userId]);

        $this->assertError($this->send('POST', '/api/minecraft/link', token: $token), 401, 'unauthorized');
        $this->assertError($this->send('GET', '/api/minecraft/link/' . bin2hex(random_bytes(16)), token: $token), 401, 'unauthorized');
        $this->assertError($this->send('GET', '/api/minecraft/accounts', token: $token), 401, 'unauthorized');
        $this->assertError($this->send('DELETE', '/api/minecraft/accounts/' . self::UUID, token: $token), 401, 'unauthorized');

        self::assertSame([], $this->requestsTo(self::DEVICE_URL));
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    private function tokenFor(int $userId): string
    {
        return JwtIssuer::fromConfig()->issue($userId, 'minecraft-route-family')['token'];
    }

    private function app(): App
    {
        if (self::$app === null) {
            self::$app = require dirname(__DIR__, 2) . '/api/bootstrap.php';
        }

        return self::$app;
    }

    /**
     * @param array<string, mixed>|null $json
     */
    private function send(string $method, string $path, ?array $json = null, ?string $token = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, $path, ['REMOTE_ADDR' => self::IP]);

        if ($json !== null) {
            $request = $request
                ->withHeader('Content-Type', 'application/json')
                ->withBody((new StreamFactory())->createStream(json_encode($json, JSON_THROW_ON_ERROR)));
        }

        if ($token !== null) {
            $request = $request->withHeader('Authorization', 'Bearer ' . $token);
        }

        return $this->app()->handle($request);
    }

    /** @return array<string, mixed> */
    private function body(ResponseInterface $response): array
    {
        $decoded = json_decode((string) $response->getBody(), true, 512, JSON_THROW_ON_ERROR);
        $this->assertIsArray($decoded);

        return $decoded;
    }

    private function assertError(ResponseInterface $response, int $status, string $code): void
    {
        $this->assertSame($status, $response->getStatusCode());
        $this->assertSame(['error' => $code], $this->body($response));
    }
}
