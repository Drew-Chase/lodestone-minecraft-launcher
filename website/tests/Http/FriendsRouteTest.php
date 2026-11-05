<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\JwtIssuer;
use app\Auth\Tokens;
use app\Friends\FriendCodes;
use app\Users\UserRepository;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;

/**
 * HTTP behaviour of /api/friends: codes, search, requests, lists, accept, remove and invites.
 */
final class FriendsRouteTest extends DatabaseTestCase
{
    private const IP = '198.51.100.70';

    private static ?App $app = null;

    public function test_the_friend_code_needs_neither_verification_nor_a_username(): void
    {
        $member = $this->member('plain@example.test', null, false);

        $first = $this->body($this->send('GET', '/api/friends/code', headers: $this->bearer($member)));
        self::assertMatchesRegularExpression('/^[ABCDEFGHJKMNPQRSTVWXYZ23456789]{8}$/', $first['code']);

        $again = $this->body($this->send('GET', '/api/friends/code', headers: $this->bearer($member)));
        self::assertSame($first['code'], $again['code']);

        $reset = $this->send('POST', '/api/friends/code/reset', headers: $this->bearer($member));
        $this->assertSame(200, $reset->getStatusCode());
        self::assertNotSame($first['code'], $this->body($reset)['code']);
    }

    public function test_search_needs_a_verified_email_and_a_username(): void
    {
        $unverified = $this->member('unverified@example.test', 'Unverified', false);
        $unnamed = $this->member('unnamed@example.test', null, true);

        $this->assertError($this->search($unverified, 'Grace'), 403, 'email_not_verified');
        $this->assertError($this->search($unnamed, 'Grace'), 409, 'username_required');
    }

    public function test_search_finds_an_account_by_username_ignoring_case(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $this->member('grace@example.test', 'Grace', true);

        $response = $this->search($ada, 'grace');

        $this->assertSame(200, $response->getStatusCode());
        self::assertSame(['user' => ['username' => 'Grace']], $this->body($response));
    }

    public function test_search_is_not_found_for_a_missing_self_or_a_malformed_name(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);

        $this->assertError($this->search($ada, 'Nobody'), 404, 'not_found');
        $this->assertError($this->search($ada, 'Ada_L'), 404, 'not_found');
        $this->assertError($this->search($ada, 'has space'), 404, 'not_found');

        $missing = $this->send('GET', '/api/friends/search', headers: $this->bearer($ada));
        $this->assertError($missing, 400, 'missing_field');
    }

    public function test_search_is_limited_to_thirty_per_fifteen_minutes_per_user(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $this->member('grace@example.test', 'Grace', true);

        for ($attempt = 0; $attempt < 30; $attempt++) {
            self::assertSame(200, $this->search($ada, 'Grace')->getStatusCode());
        }

        $this->assertError($this->search($ada, 'Grace'), 429, 'rate_limited');
    }

    public function test_a_request_by_username_or_by_code_is_pending_until_accepted(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $grace = $this->member('grace@example.test', 'Grace', true);
        $linus = $this->member('linus@example.test', 'Linus', true);

        $byName = $this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($ada));
        $this->assertSame(201, $byName->getStatusCode());
        $request = $this->body($byName)['request'];
        self::assertSame('pending', $request['status']);
        self::assertSame('Grace', $request['username']);
        self::assertIsInt($request['id']);

        $code = $this->codeFor($grace);
        $byCode = $this->send('POST', '/api/friends/requests', ['code' => strtolower($code)], $this->bearer($linus));
        $this->assertSame(201, $byCode->getStatusCode());
        self::assertSame('Grace', $this->body($byCode)['request']['username']);

        $accepted = $this->send('POST', '/api/friends/requests', ['username' => 'Ada_L'], $this->bearer($grace));
        $this->assertSame(200, $accepted->getStatusCode());
        self::assertSame('accepted', $this->body($accepted)['status']);
        self::assertSame(['username' => 'Ada_L'], $this->body($accepted)['friend']);
    }

    public function test_a_friend_code_is_matched_with_hyphens_spaces_and_any_case(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $grace = $this->member('grace@example.test', 'Grace', true);
        $linus = $this->member('linus@example.test', 'Linus', true);
        $code = $this->codeFor($grace);

        $hyphenated = substr($code, 0, 4) . '-' . substr($code, 4);
        $byHyphen = $this->send('POST', '/api/friends/requests', ['code' => strtolower($hyphenated)], $this->bearer($ada));
        $this->assertSame(201, $byHyphen->getStatusCode());
        self::assertSame('Grace', $this->body($byHyphen)['request']['username']);

        $spaced = ' ' . substr($code, 0, 4) . ' ' . substr($code, 4) . ' ';
        $bySpaces = $this->send('POST', '/api/friends/requests', ['code' => $spaced], $this->bearer($linus));
        $this->assertSame(201, $bySpaces->getStatusCode());
        self::assertSame('Grace', $this->body($bySpaces)['request']['username']);
    }

    public function test_a_code_with_a_character_outside_the_alphabet_or_the_wrong_length_is_not_found(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $this->member('grace@example.test', 'Grace', true);

        $this->assertError($this->send('POST', '/api/friends/requests', ['code' => 'ABCD-EFG0'], $this->bearer($ada)), 404, 'not_found');
        $this->assertError($this->send('POST', '/api/friends/requests', ['code' => 'ABCD-EF!H'], $this->bearer($ada)), 404, 'not_found');
        $this->assertError($this->send('POST', '/api/friends/requests', ['code' => 'ABCD-EFGHJ'], $this->bearer($ada)), 404, 'not_found');
    }

    public function test_a_request_must_name_exactly_one_target(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $this->member('grace@example.test', 'Grace', true);

        $neither = $this->send('POST', '/api/friends/requests', ['other' => 1], $this->bearer($ada));
        $this->assertError($neither, 400, 'missing_field');

        $both = $this->send('POST', '/api/friends/requests', ['username' => 'Grace', 'code' => 'ABCDEFGH'], $this->bearer($ada));
        $this->assertError($both, 400, 'missing_field');
    }

    public function test_request_errors_for_yourself_an_unknown_target_and_a_repeat(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $this->member('grace@example.test', 'Grace', true);

        $this->assertError($this->send('POST', '/api/friends/requests', ['username' => 'Ada_L'], $this->bearer($ada)), 400, 'invalid_friend');
        $this->assertError($this->send('POST', '/api/friends/requests', ['username' => 'Nobody'], $this->bearer($ada)), 404, 'not_found');
        $this->assertError($this->send('POST', '/api/friends/requests', ['code' => 'ZZZZZZZZ'], $this->bearer($ada)), 404, 'not_found');

        $this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($ada));
        $this->assertError($this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($ada)), 409, 'request_pending');
    }

    public function test_sending_a_request_needs_a_verified_email_and_a_username(): void
    {
        $this->member('grace@example.test', 'Grace', true);
        $unverified = $this->member('unverified@example.test', 'Unverified', false);
        $unnamed = $this->member('unnamed@example.test', null, true);

        $this->assertError(
            $this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($unverified)),
            403,
            'email_not_verified',
        );
        $this->assertError(
            $this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($unnamed)),
            409,
            'username_required',
        );
    }

    public function test_requests_are_limited_to_thirty_per_fifteen_minutes_per_user(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $this->member('grace@example.test', 'Grace', true);

        for ($attempt = 0; $attempt < 30; $attempt++) {
            $response = $this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($ada));
            self::assertNotSame(429, $response->getStatusCode());
        }

        $this->assertError(
            $this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($ada)),
            429,
            'rate_limited',
        );
    }

    public function test_the_list_shows_usernames_and_accept_is_for_the_addressee_only(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $grace = $this->member('grace@example.test', 'Grace', true);
        $linus = $this->member('linus@example.test', 'Linus', true);

        $request = $this->body($this->send('POST', '/api/friends/requests', ['username' => 'Grace'], $this->bearer($ada)))['request'];

        $incoming = $this->body($this->send('GET', '/api/friends', headers: $this->bearer($grace)));
        self::assertSame([['id' => $request['id'], 'username' => 'Ada_L']], $incoming['incoming']);
        self::assertSame([], $incoming['friends']);
        self::assertStringNotContainsString('ada@example.test', (string) json_encode($incoming));

        $this->assertError(
            $this->send('POST', '/api/friends/requests/' . $request['id'] . '/accept', headers: $this->bearer($ada)),
            404,
            'not_found',
        );
        $this->assertError(
            $this->send('POST', '/api/friends/requests/abc/accept', headers: $this->bearer($grace)),
            404,
            'not_found',
        );

        $accepted = $this->send('POST', '/api/friends/requests/' . $request['id'] . '/accept', headers: $this->bearer($grace));
        $this->assertSame(200, $accepted->getStatusCode());
        self::assertSame(['status' => 'accepted'], $this->body($accepted));

        $friends = $this->body($this->send('GET', '/api/friends', headers: $this->bearer($ada)))['friends'];
        self::assertSame([['id' => $request['id'], 'username' => 'Grace']], $friends);

        $stranger = $this->send('DELETE', '/api/friends/' . $request['id'], headers: $this->bearer($linus));
        $this->assertError($stranger, 404, 'not_found');

        $removed = $this->send('DELETE', '/api/friends/' . $request['id'], headers: $this->bearer($ada));
        $this->assertSame(204, $removed->getStatusCode());
        self::assertSame([], $this->body($this->send('GET', '/api/friends', headers: $this->bearer($grace)))['friends']);
    }

    public function test_accepting_needs_a_verified_email(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $unverified = $this->member('unverified@example.test', 'Unverified', false);

        $request = $this->body(
            $this->send('POST', '/api/friends/requests', ['code' => $this->codeFor($unverified)], $this->bearer($ada))
        )['request'];

        $this->assertError(
            $this->send('POST', '/api/friends/requests/' . $request['id'] . '/accept', headers: $this->bearer($unverified)),
            403,
            'email_not_verified',
        );
    }

    public function test_an_invite_link_befriends_the_inviter_once(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true);
        $grace = $this->member('grace@example.test', 'Grace', true);
        $linus = $this->member('linus@example.test', 'Linus', true);

        $created = $this->send('POST', '/api/friends/invites', headers: $this->bearer($grace));
        $this->assertSame(201, $created->getStatusCode());
        $body = $this->body($created);
        self::assertStringStartsWith('http://localhost:3000/friends/invite#token=', $body['url']);
        self::assertArrayHasKey('expires_at', $body);

        $token = substr($body['url'], strlen('http://localhost:3000/friends/invite#token='));

        $accepted = $this->send('POST', '/api/friends/invites/accept', ['token' => $token], $this->bearer($ada));
        $this->assertSame(200, $accepted->getStatusCode());
        self::assertSame(['status' => 'accepted', 'friend' => ['username' => 'Grace']], $this->body($accepted));

        $this->assertError(
            $this->send('POST', '/api/friends/invites/accept', ['token' => $token], $this->bearer($linus)),
            409,
            'invite_used',
        );
        $this->assertError(
            $this->send('POST', '/api/friends/invites/accept', ['token' => 'not-an-invite'], $this->bearer($linus)),
            404,
            'invalid_invite',
        );
        $this->assertError(
            $this->send('POST', '/api/friends/invites/accept', ['other' => 1], $this->bearer($linus)),
            400,
            'missing_field',
        );
    }

    public function test_creating_an_invite_needs_a_verified_email_and_a_username(): void
    {
        $unverified = $this->member('unverified@example.test', 'Unverified', false);
        $unnamed = $this->member('unnamed@example.test', null, true);

        $this->assertError($this->send('POST', '/api/friends/invites', headers: $this->bearer($unverified)), 403, 'email_not_verified');
        $this->assertError($this->send('POST', '/api/friends/invites', headers: $this->bearer($unnamed)), 409, 'username_required');
    }

    public function test_invite_links_are_limited_to_thirty_per_fifteen_minutes_per_user(): void
    {
        $grace = $this->member('grace@example.test', 'Grace', true);

        for ($attempt = 0; $attempt < 30; $attempt++) {
            self::assertSame(201, $this->send('POST', '/api/friends/invites', headers: $this->bearer($grace))->getStatusCode());
        }

        $this->assertError(
            $this->send('POST', '/api/friends/invites', headers: $this->bearer($grace)),
            429,
            'rate_limited',
        );
    }

    public function test_the_friend_routes_need_a_bearer_token(): void
    {
        $this->assertError($this->send('GET', '/api/friends'), 401, 'unauthorized');
        $this->assertError($this->send('GET', '/api/friends/code'), 401, 'unauthorized');
    }

    /** Creates an account, optionally with a username and a verified email, and returns its id. */
    private function member(string $email, ?string $username, bool $verified): int
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

    private function codeFor(int $userId): string
    {
        return (new FriendCodes())->forUser($userId);
    }

    private function search(int $userId, string $username): ResponseInterface
    {
        return $this->send('GET', '/api/friends/search?username=' . rawurlencode($username), headers: $this->bearer($userId));
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
