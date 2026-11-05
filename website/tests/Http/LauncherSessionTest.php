<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\AuthService;
use app\Auth\JwtIssuer;
use app\Auth\RefreshTokens;
use app\Auth\Tokens;
use app\Devices\Devices;
use app\Users\UserRepository;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * The launcher's refresh and logout on /api/auth: the token travels in the body, the rotation and reuse rules match
 * the web flow, and the device's last_seen_at follows the family.
 */
final class LauncherSessionTest extends DatabaseTestCase
{
    private const IP = '198.51.100.90';
    private const LAUNCHER = ['X-Lodestone-Client' => 'launcher'];

    private static ?App $app = null;

    private int $userId;
    private int $device;

    protected function setUp(): void
    {
        parent::setUp();

        $this->userId = (new UserRepository())->create('launcher-session@example.test', null, null);
        $this->device = (new Devices())->add($this->userId, 'Drew PC', 'Windows 11', '1.4.0');
    }

    public function test_a_launcher_refresh_returns_the_session_with_the_new_token_in_the_body_and_no_cookie(): void
    {
        $old = $this->issue();

        $response = $this->refresh($old);

        $this->assertSame(200, $response->getStatusCode());
        $body = $this->body($response);
        self::assertSame('Bearer', $body['token_type']);
        self::assertSame(900, $body['expires_in']);
        self::assertNotSame('', $body['access_token']);
        self::assertSame($this->userId, $body['user']['id']);
        self::assertMatchesRegularExpression('/^[a-f0-9]{64}$/', $body['refresh_token']);
        self::assertNotSame($old, $body['refresh_token']);
        self::assertSame([], $response->getHeader('Set-Cookie'));
    }

    public function test_a_repeat_inside_the_grace_window_is_refused_and_the_family_survives(): void
    {
        $old = $this->issue();
        $rotated = $this->body($this->refresh($old))['refresh_token'];

        $this->assertError($this->refresh($old), 401, 'refresh_invalid');

        $this->assertSame(200, $this->refresh($rotated)->getStatusCode());
    }

    public function test_a_repeat_after_the_grace_window_is_reuse_and_revokes_the_family(): void
    {
        $old = $this->issue();
        $rotated = $this->body($this->refresh($old))['refresh_token'];
        $this->backdateUse($old, 11);

        $this->assertError($this->refresh($old), 401, 'refresh_reused');
        $this->assertError($this->refresh($rotated), 401, 'refresh_invalid');
    }

    public function test_a_refresh_updates_the_last_seen_time_of_the_device(): void
    {
        $this->pdo()->prepare("UPDATE devices SET last_seen_at = '2020-01-01 00:00:00' WHERE id = ?")
            ->execute([$this->device]);

        $this->refresh($this->issue());

        $statement = $this->pdo()->prepare('SELECT last_seen_at FROM devices WHERE id = ?');
        $statement->execute([$this->device]);
        $seen = (string) $statement->fetchColumn();
        self::assertStringStartsNotWith('2020-01-01', $seen);
    }

    public function test_the_launcher_needs_a_known_client_header_and_a_body_token(): void
    {
        $this->assertError(
            $this->send('POST', '/api/auth/refresh', ['refresh_token' => $this->issue()]),
            403,
            'client_header_missing',
        );
        $this->assertError(
            $this->send('POST', '/api/auth/refresh', ['refresh_token' => $this->issue()], ['X-Lodestone-Client' => 'desktop']),
            403,
            'client_header_missing',
        );
        $this->assertError(
            $this->send('POST', '/api/auth/refresh', ['other' => 1], self::LAUNCHER),
            400,
            'missing_field',
        );
    }

    public function test_a_launcher_token_is_refused_on_the_web_cookie_route(): void
    {
        $token = $this->issue();

        $response = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: ['__Host-lodestone_refresh' => $token],
        );

        $this->assertError($response, 401, 'refresh_invalid');
    }

    public function test_a_launcher_logout_revokes_the_family_and_returns_204(): void
    {
        $token = $this->issue();

        $response = $this->send('POST', '/api/auth/logout', ['refresh_token' => $token], self::LAUNCHER);

        $this->assertSame(204, $response->getStatusCode());
        self::assertSame('', (string) $response->getBody());
        self::assertSame([], $response->getHeader('Set-Cookie'));
        $this->assertError($this->refresh($token), 401, 'refresh_invalid');
    }

    public function test_a_launcher_logout_needs_the_client_header_and_the_token(): void
    {
        $this->assertError(
            $this->send('POST', '/api/auth/logout', ['other' => 1], self::LAUNCHER),
            400,
            'missing_field',
        );
        $this->assertError(
            $this->send('POST', '/api/auth/logout', ['refresh_token' => $this->issue()]),
            403,
            'client_header_missing',
        );
    }

    public function test_a_web_token_is_refused_on_the_launcher_refresh_and_is_not_rotated(): void
    {
        $web = (new RefreshTokens())->issue($this->userId, 'web')['token'];

        $this->assertError($this->refresh($web), 401, 'refresh_invalid');

        $cookie = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: ['__Host-lodestone_refresh' => $web],
        );
        $this->assertSame(200, $cookie->getStatusCode());
    }

    public function test_a_web_token_is_refused_on_the_launcher_logout_and_is_not_revoked(): void
    {
        $web = (new RefreshTokens())->issue($this->userId, 'web')['token'];

        $this->assertError(
            $this->send('POST', '/api/auth/logout', ['refresh_token' => $web], self::LAUNCHER),
            401,
            'refresh_invalid',
        );

        $cookie = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: ['__Host-lodestone_refresh' => $web],
        );
        $this->assertSame(200, $cookie->getStatusCode());
    }

    public function test_a_launcher_logout_hides_its_device_from_the_device_list(): void
    {
        $token = $this->issue();
        $this->assertSame(1, count($this->listedDevices()));

        $this->send('POST', '/api/auth/logout', ['refresh_token' => $token], self::LAUNCHER);

        self::assertSame([], $this->listedDevices());
    }

    public function test_revoking_the_other_sessions_hides_a_launcher_device(): void
    {
        $this->issue();

        AuthService::fromConfig()->revokeOtherSessions($this->userId, Tokens::familyId(), self::IP);

        self::assertSame([], $this->listedDevices());
    }

    /** @return list<array<string, mixed>> The devices GET /api/devices lists for the user. */
    private function listedDevices(): array
    {
        $token = JwtIssuer::fromConfig()->issue($this->userId, Tokens::familyId())['token'];
        $response = $this->send('GET', '/api/devices', headers: ['Authorization' => 'Bearer ' . $token]);

        $this->assertSame(200, $response->getStatusCode());
        $body = json_decode((string) $response->getBody(), true, 512, JSON_THROW_ON_ERROR);

        return $body['devices'];
    }

    /** A launcher refresh family for the device, as the device sign-in creates it. */
    private function issue(): string
    {
        return (new RefreshTokens())->issue($this->userId, 'launcher', $this->device)['token'];
    }

    private function refresh(string $token): ResponseInterface
    {
        return $this->send('POST', '/api/auth/refresh', ['refresh_token' => $token], self::LAUNCHER);
    }

    private function backdateUse(string $token, int $seconds): void
    {
        $this->pdo()->prepare(
            'UPDATE refresh_tokens SET used_at = DATE_SUB(NOW(6), INTERVAL ' . $seconds . ' SECOND) WHERE token_hash = ?'
        )->execute([Tokens::hash($token)]);
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
     * @param array<string, string> $cookies
     */
    private function send(
        string $method,
        string $path,
        ?array $json = null,
        array $headers = [],
        array $cookies = [],
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

        return $this->app()->handle($request->withCookieParams($cookies));
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
