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
 * HTTP behaviour of POST /api/account/username, and the username on GET /api/me.
 */
final class UsernameRouteTest extends DatabaseTestCase
{
    private const IP = '198.51.100.100';

    private static ?App $app = null;

    public function test_the_username_needs_a_bearer_token(): void
    {
        $this->assertError($this->send('POST', '/api/account/username', ['username' => 'Ada_L']), 401, 'unauthorized');
    }

    public function test_a_username_that_is_missing_or_not_a_string_is_missing_field(): void
    {
        $user = $this->member('missing@example.test', false);

        $this->assertError($this->send('POST', '/api/account/username', ['other' => 1], $this->bearer($user)), 400, 'missing_field');
        $this->assertError($this->send('POST', '/api/account/username', ['username' => 123], $this->bearer($user)), 400, 'missing_field');
    }

    public function test_a_username_outside_the_rules_is_invalid(): void
    {
        $user = $this->member('rules@example.test', false);

        foreach (['ab', str_repeat('a', 17), 'has space', 'dash-name', 'dot.name', 'ünïcode', ''] as $name) {
            $this->assertError(
                $this->send('POST', '/api/account/username', ['username' => $name], $this->bearer($user)),
                400,
                'invalid_username',
            );
        }
    }

    public function test_a_valid_username_is_stored_and_shows_on_me(): void
    {
        $user = $this->member('ada@example.test', false);

        $response = $this->send('POST', '/api/account/username', ['username' => 'Ada_Lovelace'], $this->bearer($user));

        $this->assertSame(200, $response->getStatusCode());
        self::assertSame(['username' => 'Ada_Lovelace'], $this->body($response));

        $me = $this->body($this->send('GET', '/api/me', headers: $this->bearer($user)));
        self::assertSame('Ada_Lovelace', $me['user']['username']);
    }

    public function test_the_username_on_me_is_null_until_one_is_chosen(): void
    {
        $user = $this->member('fresh@example.test', false);

        $me = $this->body($this->send('GET', '/api/me', headers: $this->bearer($user)));

        self::assertArrayHasKey('username', $me['user']);
        self::assertNull($me['user']['username']);
    }

    public function test_three_and_sixteen_characters_are_both_accepted(): void
    {
        $short = $this->member('short@example.test', false);
        $long = $this->member('long@example.test', false);

        $this->assertSame(200, $this->send('POST', '/api/account/username', ['username' => 'abc'], $this->bearer($short))->getStatusCode());
        $this->assertSame(200, $this->send('POST', '/api/account/username', ['username' => str_repeat('Z', 16)], $this->bearer($long))->getStatusCode());
    }

    public function test_a_name_taken_by_another_account_is_refused_in_any_case(): void
    {
        $this->member('grace@example.test', false, 'Grace');
        $ada = $this->member('ada@example.test', false);

        $this->assertError(
            $this->send('POST', '/api/account/username', ['username' => 'grace'], $this->bearer($ada)),
            409,
            'username_taken',
        );
    }

    public function test_an_account_may_change_the_case_of_its_own_name(): void
    {
        $ada = $this->member('ada@example.test', false, 'Ada');

        $response = $this->send('POST', '/api/account/username', ['username' => 'ada'], $this->bearer($ada));

        $this->assertSame(200, $response->getStatusCode());
        self::assertSame(['username' => 'ada'], $this->body($response));
    }

    public function test_an_unverified_account_may_choose_a_username(): void
    {
        $user = $this->member('unverified@example.test', false);

        $this->assertSame(200, $this->send('POST', '/api/account/username', ['username' => 'Newcomer'], $this->bearer($user))->getStatusCode());
    }

    public function test_a_deleted_account_is_unauthorized(): void
    {
        $user = $this->member('gone@example.test', false);
        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$user]);

        $this->assertError(
            $this->send('POST', '/api/account/username', ['username' => 'Gone_Soon'], $this->bearer($user)),
            401,
            'unauthorized',
        );
    }

    /** Creates an account with an optional username. The email is verified when $verified is true. */
    private function member(string $email, bool $verified, ?string $username = null): int
    {
        $users = new UserRepository();
        $id = $users->create($email, null, null);

        if ($username !== null) {
            $users->setUsername($id, $username);
        }

        if ($verified) {
            $users->markVerified($id);
        }

        return $id;
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
    private function send(string $method, string $path, ?array $json = null, array $headers = []): ResponseInterface
    {
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
