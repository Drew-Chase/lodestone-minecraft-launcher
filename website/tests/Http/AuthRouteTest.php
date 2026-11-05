<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\Tokens;
use app\Http\Cookies;
use app\Mail\MemoryMailer;
use app\Mail\Outbox;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of the public /api/auth routes. The service rules are tested elsewhere.
 */
final class AuthRouteTest extends DatabaseTestCase
{
    private const PASSWORD = 'correct horse battery staple';
    private const NEW_PASSWORD = 'a different long passphrase';
    private const EMAIL = 'ada@example.com';
    private const IP = '198.51.100.20';

    private static ?App $app = null;

    protected function setUp(): void
    {
        parent::setUp();

        (new MemoryMailer())->reset();
        Outbox::reset();
    }

    public function test_register_returns_202_and_queues_a_verification_mail(): void
    {
        $response = $this->postJson('/api/auth/register', ['email' => self::EMAIL, 'password' => self::PASSWORD]);

        $this->assertSame(202, $response->getStatusCode());
        $this->assertSame(['status' => 'check_email'], $this->body($response));
        $this->assertNotSame([], $this->mailFor(self::EMAIL));
    }

    public function test_register_rejects_a_short_password_and_a_bad_email(): void
    {
        $short = $this->postJson('/api/auth/register', ['email' => self::EMAIL, 'password' => 'short']);
        $this->assertError($short, 400, 'password_too_short');

        $badEmail = $this->postJson('/api/auth/register', ['email' => 'not-an-address', 'password' => self::PASSWORD]);
        $this->assertError($badEmail, 400, 'invalid_email');
    }

    public function test_register_without_a_password_returns_missing_field(): void
    {
        $response = $this->postJson('/api/auth/register', ['email' => self::EMAIL]);

        $this->assertError($response, 400, 'missing_field');
    }

    public function test_verify_email_accepts_the_emailed_token_once(): void
    {
        $this->postJson('/api/auth/register', ['email' => self::EMAIL, 'password' => self::PASSWORD]);
        $token = $this->tokenFromLatestMail(self::EMAIL);

        $first = $this->postJson('/api/auth/verify-email', ['token' => $token]);
        $this->assertSame(200, $first->getStatusCode());
        $this->assertSame(['status' => 'verified'], $this->body($first));

        $second = $this->postJson('/api/auth/verify-email', ['token' => $token]);
        $this->assertError($second, 400, 'invalid_token');
    }

    public function test_login_sets_a_hardened_refresh_cookie_and_keeps_the_token_out_of_the_body(): void
    {
        $this->registerVerified(self::EMAIL);

        $response = $this->login(self::EMAIL, self::PASSWORD);

        $this->assertSame(200, $response->getStatusCode());
        $body = $this->body($response);
        $this->assertArrayHasKey('access_token', $body);
        $this->assertSame('Bearer', $body['token_type']);
        $this->assertArrayHasKey('expires_in', $body);
        $this->assertArrayHasKey('user', $body);
        $this->assertArrayNotHasKey('refresh_token', $body);
        $this->assertArrayNotHasKey('family_id', $body);

        $cookie = $this->refreshCookieLine($response);
        $this->assertStringContainsString('__Host-lodestone_refresh=', $cookie);
        $this->assertStringContainsString('; Path=/; HttpOnly; Secure; SameSite=Strict', $cookie);
        $this->assertStringContainsString('HttpOnly', $cookie);
        $this->assertStringContainsString('Secure', $cookie);
        $this->assertStringContainsString('SameSite=Strict', $cookie);
    }

    public function test_login_with_a_wrong_password_returns_401(): void
    {
        $this->registerVerified(self::EMAIL);

        $response = $this->login(self::EMAIL, 'the wrong passphrase');

        $this->assertError($response, 401, 'invalid_credentials');
    }

    public function test_five_failed_logins_return_429_with_retry_after(): void
    {
        $this->registerVerified(self::EMAIL);

        for ($attempt = 0; $attempt < 5; $attempt++) {
            $this->assertError($this->login(self::EMAIL, 'the wrong passphrase'), 401, 'invalid_credentials');
        }

        $blocked = $this->login(self::EMAIL, 'the wrong passphrase');

        $this->assertError($blocked, 429, 'rate_limited');
        $this->assertTrue($blocked->hasHeader('Retry-After'));
        $this->assertGreaterThan(0, (int) $blocked->getHeaderLine('Retry-After'));
    }

    public function test_refresh_needs_the_client_header_and_rotates_the_cookie(): void
    {
        $this->registerVerified(self::EMAIL);
        $old = $this->refreshToken($this->login(self::EMAIL, self::PASSWORD));

        $withoutHeader = $this->send('POST', '/api/auth/refresh', cookies: [Cookies::REFRESH => $old]);
        $this->assertError($withoutHeader, 403, 'client_header_missing');

        $rotated = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $old],
        );

        $this->assertSame(200, $rotated->getStatusCode());
        $body = $this->body($rotated);
        $this->assertArrayHasKey('access_token', $body);
        $this->assertArrayHasKey('user', $body);
        $this->assertArrayNotHasKey('refresh_token', $body);
        $this->assertNotSame($old, $this->refreshToken($rotated));

        // Within the grace window the repeat is refused without revoking the family.
        $repeat = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $old],
        );
        $this->assertError($repeat, 401, 'refresh_invalid');

        // Past the window, the same token is reuse, which revokes the family.
        $backdate = $this->pdo()->prepare(
            'UPDATE refresh_tokens SET used_at = DATE_SUB(NOW(6), INTERVAL 11 SECOND) WHERE token_hash = ?'
        );
        $backdate->execute([Tokens::hash($old)]);

        $reused = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $old],
        );
        $this->assertError($reused, 401, 'refresh_reused');
    }

    public function test_logout_returns_204_and_clears_the_cookie(): void
    {
        $this->registerVerified(self::EMAIL);
        $session = $this->login(self::EMAIL, self::PASSWORD);

        $response = $this->send(
            'POST',
            '/api/auth/logout',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $this->refreshToken($session)],
        );

        $this->assertSame(204, $response->getStatusCode());
        $this->assertSame('', (string) $response->getBody());
        $this->assertStringContainsString('Max-Age=0', $this->refreshCookieLine($response));

        $absent = $this->send('POST', '/api/auth/logout', headers: ['X-Lodestone-Client' => 'web']);
        $this->assertSame(204, $absent->getStatusCode());
    }

    public function test_forgot_password_answers_the_same_for_known_and_unknown_addresses(): void
    {
        $this->registerVerified(self::EMAIL);

        $known = $this->postJson('/api/auth/forgot-password', ['email' => self::EMAIL]);
        $unknown = $this->postJson('/api/auth/forgot-password', ['email' => 'nobody@example.com']);

        $this->assertSame(202, $known->getStatusCode());
        $this->assertSame(202, $unknown->getStatusCode());
        $this->assertSame(['status' => 'check_email'], $this->body($known));
        $this->assertSame($this->body($known), $this->body($unknown));
    }

    public function test_reset_password_with_the_emailed_token_changes_the_password_and_ends_sessions(): void
    {
        $this->registerVerified(self::EMAIL);
        $session = $this->login(self::EMAIL, self::PASSWORD);
        $oldCookie = $this->refreshToken($session);

        $this->postJson('/api/auth/forgot-password', ['email' => self::EMAIL]);
        $token = $this->tokenFromLatestMail(self::EMAIL);

        $reset = $this->postJson('/api/auth/reset-password', ['token' => $token, 'password' => self::NEW_PASSWORD]);
        $this->assertSame(200, $reset->getStatusCode());
        $this->assertSame(['status' => 'reset'], $this->body($reset));

        $this->assertSame(200, $this->login(self::EMAIL, self::NEW_PASSWORD)->getStatusCode());
        $this->assertError($this->login(self::EMAIL, self::PASSWORD), 401, 'invalid_credentials');

        $stale = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $oldCookie],
        );
        $this->assertError($stale, 401, 'refresh_invalid');
    }

    public function test_reset_password_rejects_a_missing_token(): void
    {
        $response = $this->postJson('/api/auth/reset-password', ['password' => self::NEW_PASSWORD]);

        $this->assertError($response, 400, 'invalid_token');
    }

    public function test_reset_password_without_a_password_returns_missing_field(): void
    {
        $response = $this->postJson('/api/auth/reset-password', ['token' => str_repeat('a', 64)]);

        $this->assertError($response, 400, 'missing_field');
    }

    public function test_a_body_that_is_not_json_returns_400(): void
    {
        $response = $this->send('POST', '/api/auth/login', raw: 'not json');

        $this->assertError($response, 400, 'invalid_json');
    }

    public function test_a_json_array_body_returns_400_invalid_json(): void
    {
        $response = $this->send('POST', '/api/auth/login', raw: '[]');

        $this->assertError($response, 400, 'invalid_json');
    }

    public function test_an_empty_json_object_to_login_returns_401_invalid_credentials(): void
    {
        $response = $this->send('POST', '/api/auth/login', raw: '{}');

        $this->assertError($response, 401, 'invalid_credentials');
    }

    public function test_an_unknown_path_returns_404_and_a_wrong_method_returns_405(): void
    {
        $this->assertError($this->send('GET', '/api/no-such-route'), 404, 'not_found');
        $this->assertError($this->send('GET', '/api/auth/login'), 405, 'method_not_allowed');
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
     */
    private function postJson(string $path, array $json): ResponseInterface
    {
        return $this->send('POST', $path, json: $json);
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

    /** The Set-Cookie line for the refresh cookie, or '' when none was set. */
    private function refreshCookieLine(ResponseInterface $response): string
    {
        foreach ($response->getHeader('Set-Cookie') as $line) {
            if (str_starts_with($line, '__Host-lodestone_refresh=')) {
                return $line;
            }
        }

        return '';
    }

    private function refreshToken(ResponseInterface $response): string
    {
        $this->assertSame(1, preg_match('/^__Host-lodestone_refresh=([a-f0-9]{64});/', $this->refreshCookieLine($response), $matches));

        return $matches[1];
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
