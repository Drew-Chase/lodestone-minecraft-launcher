<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\JwtIssuer;
use app\Auth\Tokens;
use app\Users\UserRepository;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of the device sign-in routes and the device list: /api/device/*, /api/devices.
 */
final class DeviceRouteTest extends DatabaseTestCase
{
    private const IP = '198.51.100.60';
    private const EMAIL = 'launcher@example.test';

    private static ?App $app = null;

    /** @var array{name: string, os: string, app_version: string} */
    private const START = ['name' => 'Drew PC', 'os' => 'Windows 11', 'app_version' => '1.4.0'];

    public function test_start_returns_the_codes_and_the_verification_links(): void
    {
        $response = $this->send('POST', '/api/device/start', self::START);

        $this->assertSame(200, $response->getStatusCode());
        $body = $this->body($response);
        self::assertMatchesRegularExpression('/^[A-Za-z0-9_-]{43}$/', $body['device_code']);
        self::assertMatchesRegularExpression('/^[ABCDEFGHJKMNPQRSTVWXYZ23456789]{4}-[ABCDEFGHJKMNPQRSTVWXYZ23456789]{4}$/', $body['user_code']);
        self::assertSame('http://localhost:3000/link', $body['verification_uri']);
        self::assertSame('http://localhost:3000/link?code=' . $body['user_code'], $body['verification_uri_complete']);
        self::assertSame(600, $body['expires_in']);
        self::assertSame(5, $body['interval']);
    }

    public function test_start_without_a_field_is_missing_field_and_out_of_range_is_invalid_device(): void
    {
        $this->assertError($this->send('POST', '/api/device/start', ['os' => 'Windows', 'app_version' => '1']), 400, 'missing_field');
        $this->assertError($this->send('POST', '/api/device/start', array_merge(self::START, ['os' => 123])), 400, 'missing_field');

        $this->assertError($this->send('POST', '/api/device/start', ['name' => str_repeat('n', 65), 'os' => 'Windows', 'app_version' => '1']), 400, 'invalid_device');
        $this->assertError($this->send('POST', '/api/device/start', ['name' => '   ', 'os' => 'Windows', 'app_version' => '1']), 400, 'invalid_device');
        $this->assertError($this->send('POST', '/api/device/start', ['name' => "bad\x01name", 'os' => 'Windows', 'app_version' => '1']), 400, 'invalid_device');
        $this->assertError($this->send('POST', '/api/device/start', array_merge(self::START, ['os' => str_repeat('o', 33)])), 400, 'invalid_device');
        $this->assertError($this->send('POST', '/api/device/start', array_merge(self::START, ['app_version' => ''])), 400, 'invalid_device');
    }

    public function test_start_is_limited_to_ten_per_client_address_per_fifteen_minutes(): void
    {
        for ($attempt = 0; $attempt < 10; $attempt++) {
            self::assertSame(200, $this->send('POST', '/api/device/start', self::START)->getStatusCode());
        }

        $blocked = $this->send('POST', '/api/device/start', self::START);

        $this->assertError($blocked, 429, 'rate_limited');
        self::assertGreaterThan(0, (int) $blocked->getHeaderLine('Retry-After'));
    }

    public function test_the_token_endpoint_reports_pending_then_slow_down_then_the_session_after_approval(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);
        $started = $this->body($this->send('POST', '/api/device/start', self::START));

        $pending = $this->send('POST', '/api/device/token', ['device_code' => $started['device_code']]);
        $this->assertError($pending, 400, 'authorization_pending');

        $slow = $this->send('POST', '/api/device/token', ['device_code' => $started['device_code']]);
        $this->assertError($slow, 400, 'slow_down');
        self::assertSame('5', $slow->getHeaderLine('Retry-After'));

        $approved = $this->send(
            'POST',
            '/api/device/approve',
            ['user_code' => strtolower(str_replace('-', '', $started['user_code']))],
            $this->bearer($userId),
        );
        $this->assertSame(200, $approved->getStatusCode());
        self::assertSame(['status' => 'approved'], $this->body($approved));

        $session = $this->send('POST', '/api/device/token', ['device_code' => $started['device_code']]);

        $this->assertSame(200, $session->getStatusCode());
        $body = $this->body($session);
        self::assertSame('Bearer', $body['token_type']);
        self::assertSame(900, $body['expires_in']);
        self::assertMatchesRegularExpression('/^[a-f0-9]{64}$/', $body['refresh_token']);
        self::assertSame('Drew PC', $body['device']['name']);
        self::assertSame(self::EMAIL, $body['user']['email']);
        self::assertArrayNotHasKey('password_hash', $body['user']);
        self::assertSame([], $session->getHeader('Set-Cookie'));

        $me = $this->send('GET', '/api/me', headers: ['Authorization' => 'Bearer ' . $body['access_token']]);
        $this->assertSame(200, $me->getStatusCode());

        $this->assertError($this->send('POST', '/api/device/token', ['device_code' => $started['device_code']]), 400, 'expired_token');
    }

    public function test_token_without_a_device_code_is_missing_field_and_an_unknown_one_is_invalid(): void
    {
        $this->assertError($this->send('POST', '/api/device/token', ['x' => 1]), 400, 'missing_field');
        $this->assertError($this->send('POST', '/api/device/token', ['device_code' => 'nope']), 400, 'invalid_device_code');
    }

    public function test_approve_needs_a_bearer_token_and_a_user_code(): void
    {
        $this->assertError($this->send('POST', '/api/device/approve', ['user_code' => 'AAAA-AAAA']), 401, 'unauthorized');

        $userId = (new UserRepository())->create(self::EMAIL, null, null);

        $this->assertError($this->send('POST', '/api/device/approve', ['other' => 1], $this->bearer($userId)), 400, 'missing_field');
        $this->assertError($this->send('POST', '/api/device/approve', ['user_code' => 'AAAA-AAAA'], $this->bearer($userId)), 404, 'invalid_code');
    }

    public function test_approve_is_limited_to_thirty_per_user_per_fifteen_minutes(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);

        for ($attempt = 0; $attempt < 30; $attempt++) {
            $this->assertError($this->send('POST', '/api/device/approve', ['user_code' => 'AAAA-AAAA'], $this->bearer($userId)), 404, 'invalid_code');
        }

        $blocked = $this->send('POST', '/api/device/approve', ['user_code' => 'AAAA-AAAA'], $this->bearer($userId));

        $this->assertError($blocked, 429, 'rate_limited');
        self::assertGreaterThan(0, (int) $blocked->getHeaderLine('Retry-After'));
    }

    public function test_lookup_returns_the_details_of_a_pending_sign_in_and_no_secret(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);
        $started = $this->body($this->send('POST', '/api/device/start', self::START));

        $response = $this->send('POST', '/api/device/lookup', ['user_code' => $started['user_code']], $this->bearer($userId));

        $this->assertSame(200, $response->getStatusCode());
        $raw = (string) $response->getBody();
        $body = json_decode($raw, true, 512, JSON_THROW_ON_ERROR);
        self::assertSame(['name', 'os', 'app_version', 'expires_at'], array_keys($body));
        self::assertSame('Drew PC', $body['name']);
        self::assertSame('Windows 11', $body['os']);
        self::assertSame('1.4.0', $body['app_version']);
        self::assertMatchesRegularExpression('/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00$/', $body['expires_at']);
        self::assertStringNotContainsString($started['device_code'], $raw);
        self::assertStringNotContainsString(Tokens::hash($started['device_code']), $raw);
    }

    public function test_lookup_of_an_approved_or_consumed_code_is_code_used(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);
        $started = $this->body($this->send('POST', '/api/device/start', self::START));
        $this->send('POST', '/api/device/approve', ['user_code' => $started['user_code']], $this->bearer($userId));

        $this->assertError(
            $this->send('POST', '/api/device/lookup', ['user_code' => $started['user_code']], $this->bearer($userId)),
            409,
            'code_used',
        );

        $this->send('POST', '/api/device/token', ['device_code' => $started['device_code']]);

        $this->assertError(
            $this->send('POST', '/api/device/lookup', ['user_code' => $started['user_code']], $this->bearer($userId)),
            409,
            'code_used',
        );
    }

    public function test_lookup_of_an_expired_code_is_code_expired(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);
        $started = $this->body($this->send('POST', '/api/device/start', self::START));
        $this->pdo()->prepare('UPDATE device_links SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE user_code = ?')
            ->execute([str_replace('-', '', $started['user_code'])]);

        $this->assertError(
            $this->send('POST', '/api/device/lookup', ['user_code' => $started['user_code']], $this->bearer($userId)),
            410,
            'code_expired',
        );
    }

    public function test_lookup_of_an_unknown_code_is_invalid_code_and_needs_a_code_and_a_bearer_token(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);

        $this->assertError($this->send('POST', '/api/device/lookup', ['user_code' => 'AAAA-AAAA']), 401, 'unauthorized');
        $this->assertError($this->send('POST', '/api/device/lookup', ['other' => 1], $this->bearer($userId)), 400, 'missing_field');
        $this->assertError($this->send('POST', '/api/device/lookup', ['user_code' => 'AAAA-AAAA'], $this->bearer($userId)), 404, 'invalid_code');
        $this->assertError($this->send('POST', '/api/device/lookup', ['user_code' => 'nope'], $this->bearer($userId)), 404, 'invalid_code');
    }

    public function test_lookup_is_limited_to_thirty_per_user_per_fifteen_minutes(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);

        for ($attempt = 0; $attempt < 30; $attempt++) {
            $this->assertError($this->send('POST', '/api/device/lookup', ['user_code' => 'AAAA-AAAA'], $this->bearer($userId)), 404, 'invalid_code');
        }

        $blocked = $this->send('POST', '/api/device/lookup', ['user_code' => 'AAAA-AAAA'], $this->bearer($userId));

        $this->assertError($blocked, 429, 'rate_limited');
        self::assertGreaterThan(0, (int) $blocked->getHeaderLine('Retry-After'));
    }

    public function test_the_device_list_shows_active_devices_with_the_plan_limit(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);
        $device = $this->signIn($userId)['device']['id'];

        $response = $this->send('GET', '/api/devices', headers: $this->bearer($userId));

        $this->assertSame(200, $response->getStatusCode());
        $body = $this->body($response);
        self::assertSame(1, $body['limit']);
        self::assertCount(1, $body['devices']);
        self::assertSame(
            ['id', 'name', 'os', 'app_version', 'created_at', 'last_seen_at'],
            array_keys($body['devices'][0])
        );
        self::assertSame($device, $body['devices'][0]['id']);
    }

    public function test_revoking_a_device_ends_its_refresh_family_and_hides_it(): void
    {
        $userId = (new UserRepository())->create(self::EMAIL, null, null);
        $session = $this->signIn($userId);
        $device = $session['device']['id'];
        $refresh = $session['refresh_token'];

        $revoked = $this->send('DELETE', '/api/devices/' . $device, headers: $this->bearer($userId));
        $this->assertSame(204, $revoked->getStatusCode());

        $listed = $this->body($this->send('GET', '/api/devices', headers: $this->bearer($userId)));
        self::assertSame([], $listed['devices']);

        $refreshed = $this->send(
            'POST',
            '/api/auth/refresh',
            ['refresh_token' => $refresh],
            ['X-Lodestone-Client' => 'launcher'],
        );
        $this->assertError($refreshed, 401, 'refresh_invalid');

        $this->assertError($this->send('DELETE', '/api/devices/' . $device, headers: $this->bearer($userId)), 404, 'not_found');
    }

    public function test_a_device_of_another_account_or_a_malformed_id_is_not_found(): void
    {
        $owner = (new UserRepository())->create('owner@example.test', null, null);
        $stranger = (new UserRepository())->create('stranger@example.test', null, null);
        $device = $this->signIn($owner)['device']['id'];

        $this->assertError($this->send('DELETE', '/api/devices/' . $device, headers: $this->bearer($stranger)), 404, 'not_found');
        $this->assertError($this->send('DELETE', '/api/devices/abc', headers: $this->bearer($owner)), 404, 'not_found');
    }

    public function test_the_device_routes_need_a_bearer_token(): void
    {
        $this->assertError($this->send('GET', '/api/devices'), 401, 'unauthorized');
        $this->assertError($this->send('DELETE', '/api/devices/1'), 401, 'unauthorized');
    }

    /**
     * Runs the whole device sign-in for the user and returns the session body.
     *
     * @return array<string, mixed>
     */
    private function signIn(int $userId): array
    {
        $started = $this->body($this->send('POST', '/api/device/start', self::START));
        $this->send('POST', '/api/device/approve', ['user_code' => $started['user_code']], $this->bearer($userId));

        return $this->body($this->send('POST', '/api/device/token', ['device_code' => $started['device_code']]));
    }

    /** @return array<string, string> */
    private function bearer(int $userId): array
    {
        return ['Authorization' => 'Bearer ' . JwtIssuer::fromConfig()->issue($userId, Tokens::familyId())['token']];
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
     * @param array<string, string> $headers
     */
    private function send(
        string $method,
        string $path,
        ?array $json = null,
        array $headers = [],
    ): ResponseInterface {
        $request = (new ServerRequestFactory())->createServerRequest($method, $path, ['REMOTE_ADDR' => self::IP]);

        if ($json !== null) {
            $request = $request
                ->withHeader('Content-Type', 'application/json')
                ->withBody((new StreamFactory())->createStream(json_encode($json, JSON_THROW_ON_ERROR)));
        }

        foreach ($headers as $name => $value) {
            $request = $request->withHeader($name, $value);
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
