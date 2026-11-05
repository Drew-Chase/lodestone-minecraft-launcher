<?php

declare(strict_types=1);

namespace tests\Http;

use app\Http\Cookies;
use app\Mail\MemoryMailer;
use app\Mail\Outbox;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of the bearer-protected routes: GET /api/me and POST /api/auth/change-password.
 */
final class AccountRouteTest extends DatabaseTestCase
{
    private const PASSWORD = 'correct horse battery staple';
    private const NEW_PASSWORD = 'a different long passphrase';
    private const EMAIL = 'grace@example.com';
    private const IP = '198.51.100.30';

    private static ?App $app = null;

    protected function setUp(): void
    {
        parent::setUp();

        (new MemoryMailer())->reset();
        Outbox::reset();
    }

    public function test_me_returns_the_user_for_a_valid_bearer_token(): void
    {
        $this->registerVerified(self::EMAIL);
        $access = $this->accessToken($this->login(self::EMAIL, self::PASSWORD));

        $response = $this->send('GET', '/api/me', headers: ['Authorization' => 'Bearer ' . $access]);

        $this->assertSame(200, $response->getStatusCode());
        $body = $this->body($response);
        $this->assertSame(self::EMAIL, $body['user']['email']);
        $this->assertArrayNotHasKey('password_hash', $body['user']);
    }

    public function test_me_without_a_token_or_with_a_forged_token_returns_401(): void
    {
        $missing = $this->send('GET', '/api/me');
        $this->assertError($missing, 401, 'unauthorized');

        $forged = $this->send(
            'GET',
            '/api/me',
            headers: ['Authorization' => 'Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.invalid'],
        );
        $this->assertError($forged, 401, 'unauthorized');
    }

    public function test_change_password_without_a_bearer_token_returns_401(): void
    {
        $response = $this->postJson('/api/auth/change-password', [
            'current_password' => self::PASSWORD,
            'new_password' => self::NEW_PASSWORD,
        ]);

        $this->assertError($response, 401, 'unauthorized');
    }

    public function test_change_password_with_the_right_current_password_succeeds_and_ends_other_sessions(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL, self::PASSWORD);
        $other = $this->login(self::EMAIL, self::PASSWORD);

        $response = $this->postJson(
            '/api/auth/change-password',
            ['current_password' => self::PASSWORD, 'new_password' => self::NEW_PASSWORD],
            $this->bearer($current),
        );

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['status' => 'changed'], $this->body($response));

        $this->assertSame(200, $this->login(self::EMAIL, self::NEW_PASSWORD)->getStatusCode());

        $stale = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $this->refreshToken($other)],
        );
        $this->assertError($stale, 401, 'refresh_invalid');
    }

    public function test_change_password_with_a_wrong_current_password_returns_401(): void
    {
        $this->registerVerified(self::EMAIL);
        $session = $this->login(self::EMAIL, self::PASSWORD);

        $response = $this->postJson(
            '/api/auth/change-password',
            ['current_password' => 'the wrong passphrase', 'new_password' => self::NEW_PASSWORD],
            $this->bearer($session),
        );

        $this->assertError($response, 401, 'invalid_credentials');
    }

    public function test_change_password_rejects_a_short_new_password(): void
    {
        $this->registerVerified(self::EMAIL);
        $session = $this->login(self::EMAIL, self::PASSWORD);

        $response = $this->postJson(
            '/api/auth/change-password',
            ['current_password' => self::PASSWORD, 'new_password' => 'short'],
            $this->bearer($session),
        );

        $this->assertError($response, 400, 'password_too_short');
    }

    public function test_change_password_without_a_current_password_returns_missing_field(): void
    {
        $this->registerVerified(self::EMAIL);
        $session = $this->login(self::EMAIL, self::PASSWORD);

        $response = $this->postJson(
            '/api/auth/change-password',
            ['new_password' => self::NEW_PASSWORD],
            $this->bearer($session),
        );

        $this->assertError($response, 400, 'missing_field');
    }

    public function test_change_password_without_a_new_password_returns_missing_field(): void
    {
        $this->registerVerified(self::EMAIL);
        $session = $this->login(self::EMAIL, self::PASSWORD);

        $response = $this->postJson(
            '/api/auth/change-password',
            ['current_password' => self::PASSWORD],
            $this->bearer($session),
        );

        $this->assertError($response, 400, 'missing_field');
    }

    private function app(): App
    {
        if (self::$app === null) {
            self::$app = require dirname(__DIR__, 2) . '/api/bootstrap.php';
        }

        return self::$app;
    }

    /**
     * @param array<string, mixed> $json
     * @param array<string, string> $headers
     */
    private function postJson(string $path, array $json, array $headers = []): ResponseInterface
    {
        return $this->send('POST', $path, json: $json, headers: $headers);
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $cookies
     * @param array<string, mixed>|null $json
     */
    private function send(
        string $method,
        string $path,
        ?array $json = null,
        ?string $raw = null,
        array $headers = [],
        array $cookies = [],
    ): ResponseInterface {
        $request = (new ServerRequestFactory())->createServerRequest($method, $path, ['REMOTE_ADDR' => self::IP]);

        $payload = $raw ?? ($json === null ? null : json_encode($json, JSON_THROW_ON_ERROR));
        if ($payload !== null) {
            $request = $request
                ->withHeader('Content-Type', 'application/json')
                ->withBody((new StreamFactory())->createStream($payload));
        }

        foreach ($headers as $name => $value) {
            $request = $request->withHeader($name, $value);
        }

        $request = $request->withCookieParams($cookies);

        return $this->app()->handle($request);
    }

    private function login(string $email, string $password): ResponseInterface
    {
        return $this->postJson('/api/auth/login', ['email' => $email, 'password' => $password]);
    }

    /** Registers the address, then verifies it through the emailed link. */
    private function registerVerified(string $email): void
    {
        $this->postJson('/api/auth/register', ['email' => $email, 'password' => self::PASSWORD]);
        $token = $this->tokenFromLatestMail($email);

        $this->assertSame(200, $this->postJson('/api/auth/verify-email', ['token' => $token])->getStatusCode());
    }

    /**
     * Sends the queued mail, then returns the messages sent to the address.
     *
     * @return list<array{to: string, subject: string, text: string, html: string}>
     */
    private function mailFor(string $email): array
    {
        Outbox::flush(static fn () => new MemoryMailer());

        return array_values(array_filter(
            (new MemoryMailer())->sent(),
            static fn (array $message): bool => $message['to'] === $email,
        ));
    }

    private function tokenFromLatestMail(string $email): string
    {
        $messages = $this->mailFor($email);
        $this->assertNotSame([], $messages, 'No mail was sent to the address');

        $latest = end($messages);
        $this->assertSame(1, preg_match('/token=([a-f0-9]{64})/', $latest['text'], $matches));

        return $matches[1];
    }

    /** @return array<string, string> */
    private function bearer(ResponseInterface $session): array
    {
        return ['Authorization' => 'Bearer ' . $this->accessToken($session)];
    }

    private function accessToken(ResponseInterface $session): string
    {
        $body = $this->body($session);
        $this->assertIsString($body['access_token'] ?? null);

        return $body['access_token'];
    }

    /** The refresh token from the Set-Cookie header of a login or refresh response. */
    private function refreshToken(ResponseInterface $response): string
    {
        foreach ($response->getHeader('Set-Cookie') as $line) {
            if (preg_match('/^__Host-lodestone_refresh=([a-f0-9]{64});/', $line, $matches) === 1) {
                return $matches[1];
            }
        }

        $this->fail('No refresh cookie was set');
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
