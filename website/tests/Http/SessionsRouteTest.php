<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\JwtIssuer;
use app\Http\Cookies;
use app\Mail\MemoryMailer;
use app\Mail\Outbox;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of the bearer-protected session routes: GET /api/account/sessions,
 * DELETE /api/account/sessions/{id} and POST /api/account/sessions/revoke-others.
 */
final class SessionsRouteTest extends DatabaseTestCase
{
    private const PASSWORD = 'correct horse battery staple';
    private const EMAIL = 'hopper@example.com';
    private const IP = '198.51.100.31';
    private const UNKNOWN_FAMILY = 'ffffffffffffffffffffffffffffffff';

    private static ?App $app = null;

    protected function setUp(): void
    {
        parent::setUp();

        (new MemoryMailer())->reset();
        Outbox::reset();
    }

    public function test_get_lists_the_sessions_and_flags_the_current_one(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL);
        $this->login(self::EMAIL);

        $response = $this->send('GET', '/api/account/sessions', headers: $this->bearer($current));

        $this->assertSame(200, $response->getStatusCode());
        $sessions = $this->body($response)['sessions'];
        $this->assertCount(2, $sessions);

        $flagged = array_values(array_filter($sessions, static fn (array $session): bool => $session['current'] === true));
        $this->assertCount(1, $flagged);
        $this->assertSame($this->familyId($current), $flagged[0]['id']);
        $this->assertSame('web', $flagged[0]['client']);
    }

    public function test_delete_of_another_session_returns_204_and_ends_it(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL);
        $other = $this->login(self::EMAIL);

        $response = $this->send(
            'DELETE',
            '/api/account/sessions/' . $this->familyId($other),
            headers: $this->bearer($current),
        );

        $this->assertSame(204, $response->getStatusCode());
        $this->assertSame('', (string) $response->getBody());

        $stale = $this->send(
            'POST',
            '/api/auth/refresh',
            headers: ['X-Lodestone-Client' => 'web'],
            cookies: [Cookies::REFRESH => $this->refreshToken($other)],
        );
        $this->assertError($stale, 401, 'refresh_invalid');
    }

    public function test_delete_of_the_current_session_returns_409(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL);
        $this->login(self::EMAIL);

        $response = $this->send(
            'DELETE',
            '/api/account/sessions/' . $this->familyId($current),
            headers: $this->bearer($current),
        );

        $this->assertError($response, 409, 'cannot_revoke_current');
    }

    public function test_delete_with_a_malformed_id_returns_404(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL);

        $response = $this->send('DELETE', '/api/account/sessions/not-a-session', headers: $this->bearer($current));

        $this->assertError($response, 404, 'not_found');
    }

    public function test_delete_of_an_unknown_id_returns_404(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL);

        $response = $this->send(
            'DELETE',
            '/api/account/sessions/' . self::UNKNOWN_FAMILY,
            headers: $this->bearer($current),
        );

        $this->assertError($response, 404, 'not_found');
    }

    public function test_revoke_others_returns_the_count_and_keeps_the_current_session(): void
    {
        $this->registerVerified(self::EMAIL);
        $current = $this->login(self::EMAIL);
        $this->login(self::EMAIL);
        $this->login(self::EMAIL);

        $response = $this->send('POST', '/api/account/sessions/revoke-others', headers: $this->bearer($current));

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(['status' => 'revoked', 'count' => 2], $this->body($response));

        $listed = $this->send('GET', '/api/account/sessions', headers: $this->bearer($current));
        $this->assertCount(1, $this->body($listed)['sessions']);
    }

    public function test_each_session_route_without_a_bearer_token_returns_401(): void
    {
        $list = $this->send('GET', '/api/account/sessions');
        $this->assertError($list, 401, 'unauthorized');

        $revoke = $this->send('DELETE', '/api/account/sessions/' . self::UNKNOWN_FAMILY);
        $this->assertError($revoke, 401, 'unauthorized');

        $others = $this->send('POST', '/api/account/sessions/revoke-others');
        $this->assertError($others, 401, 'unauthorized');
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

    /** Logs in and returns the login response, which carries the access token and sets the refresh cookie. */
    private function login(string $email): ResponseInterface
    {
        return $this->send(
            'POST',
            '/api/auth/login',
            json: ['email' => $email, 'password' => self::PASSWORD],
        );
    }

    /** Registers the address, then verifies it through the emailed link. */
    private function registerVerified(string $email): void
    {
        $this->send('POST', '/api/auth/register', json: ['email' => $email, 'password' => self::PASSWORD]);
        $token = $this->tokenFromLatestMail($email);

        $this->assertSame(
            200,
            $this->send('POST', '/api/auth/verify-email', json: ['token' => $token])->getStatusCode(),
        );
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

    /** The family id carried by the session's access token. */
    private function familyId(ResponseInterface $session): string
    {
        return JwtIssuer::fromConfig()->verify($this->accessToken($session))['family_id'];
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
