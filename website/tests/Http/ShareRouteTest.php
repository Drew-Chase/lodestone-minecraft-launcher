<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\JwtIssuer;
use app\Auth\Tokens;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Support\DatabaseTestCase;
use tests\Sync\ShareFixtures;

/**
 * HTTP behaviour of /api/instances/{id}/shares, and the friendship-removal hook that ends shares between the pair.
 */
final class ShareRouteTest extends DatabaseTestCase
{
    private const IP = '198.51.100.80';

    private static ?App $app = null;

    public function test_the_owner_creates_a_share_then_updates_it_with_the_same_body_shape(): void
    {
        [$owner, $friend, $instance] = $this->sharedWorld();

        $created = $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'viewer'], $this->bearer($owner));
        self::assertSame(201, $created->getStatusCode());

        $updated = $this->send('POST', "/api/instances/$instance/shares", ['username' => 'friend_1', 'access' => 'editor'], $this->bearer($owner));
        self::assertSame(200, $updated->getStatusCode());

        $share = $this->body($updated)['share'];
        self::assertSame(['username', 'access', 'granted_at'], array_keys($share));
        self::assertSame('Friend_1', $share['username']);
        self::assertSame('editor', $share['access']);
        self::assertMatchesRegularExpression('/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2}$/', $share['granted_at']);
    }

    public function test_the_first_grant_returns_created_with_the_share(): void
    {
        [$owner] = $this->sharedWorld();
        $instance = ShareFixtures::instance($this->pdo(), $owner, 'Creative');

        $response = $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'viewer'], $this->bearer($owner));

        $this->assertSame(201, $response->getStatusCode());
        self::assertSame('viewer', $this->body($response)['share']['access']);
    }

    public function test_updating_a_share_at_the_cap_is_allowed_and_a_new_instance_is_refused(): void
    {
        [$owner] = $this->sharedWorld();
        $instances = [];
        for ($i = 0; $i < 6; $i++) {
            $instances[] = ShareFixtures::instance($this->pdo(), $owner, 'World ' . $i);
        }
        for ($i = 0; $i < 5; $i++) {
            $this->grantTo($owner, $instances[$i], 'Friend_1', 'viewer');
        }

        $updated = $this->send('POST', "/api/instances/{$instances[0]}/shares", ['username' => 'Friend_1', 'access' => 'editor'], $this->bearer($owner));
        $this->assertSame(200, $updated->getStatusCode());

        $this->assertError(
            $this->send('POST', "/api/instances/{$instances[5]}/shares", ['username' => 'Friend_1', 'access' => 'viewer'], $this->bearer($owner)),
            409,
            'share_limit_reached',
        );
    }

    public function test_the_list_returns_the_shares_of_an_instance(): void
    {
        [$owner, $friend, $instance] = $this->sharedWorld();
        $this->grantTo($owner, $instance, 'Friend_1', 'viewer');

        $response = $this->send('GET', "/api/instances/$instance/shares", headers: $this->bearer($owner));

        $this->assertSame(200, $response->getStatusCode());
        $shares = $this->body($response)['shares'];
        self::assertCount(1, $shares);
        self::assertSame(['username', 'access', 'granted_at'], array_keys($shares[0]));
        self::assertSame('Friend_1', $shares[0]['username']);
        self::assertSame('viewer', $shares[0]['access']);
    }

    public function test_the_owner_removes_a_share_by_username_ignoring_case(): void
    {
        [$owner, $friend, $instance] = $this->sharedWorld();
        $this->grantTo($owner, $instance, 'Friend_1', 'viewer');

        $removed = $this->send('DELETE', "/api/instances/$instance/shares/friend_1", headers: $this->bearer($owner));
        $this->assertSame(204, $removed->getStatusCode());
        self::assertSame(['shares' => []], $this->body($this->send('GET', "/api/instances/$instance/shares", headers: $this->bearer($owner))));

        $this->assertError(
            $this->send('DELETE', "/api/instances/$instance/shares/Friend_1", headers: $this->bearer($owner)),
            404,
            'not_found',
        );
    }

    public function test_an_editor_or_a_viewer_cannot_use_any_share_endpoint(): void
    {
        [$owner, $friend, $instance] = $this->sharedWorld();
        $this->grantTo($owner, $instance, 'Friend_1', 'viewer');
        $editor = ShareFixtures::user('editor@example.test', 'Editor_1');
        $viewer = ShareFixtures::user('viewer@example.test', 'Viewer_1');
        ShareFixtures::share($this->pdo(), $instance, $editor, 'editor');
        ShareFixtures::share($this->pdo(), $instance, $viewer, 'viewer');

        foreach ([$editor, $viewer] as $member) {
            $this->assertError(
                $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'editor'], $this->bearer($member)),
                403,
                'forbidden',
            );
            $this->assertError($this->send('GET', "/api/instances/$instance/shares", headers: $this->bearer($member)), 403, 'forbidden');
            $this->assertError(
                $this->send('DELETE', "/api/instances/$instance/shares/Friend_1", headers: $this->bearer($member)),
                403,
                'forbidden',
            );
        }
    }

    public function test_an_account_with_no_role_on_the_instance_gets_not_found(): void
    {
        [$owner, $friend, $instance] = $this->sharedWorld();
        $this->grantTo($owner, $instance, 'Friend_1', 'viewer');
        $stranger = ShareFixtures::user('stranger@example.test', 'Stranger');

        $this->assertError(
            $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'editor'], $this->bearer($stranger)),
            404,
            'not_found',
        );
        $this->assertError($this->send('GET', "/api/instances/$instance/shares", headers: $this->bearer($stranger)), 404, 'not_found');
        $this->assertError(
            $this->send('DELETE', "/api/instances/$instance/shares/Friend_1", headers: $this->bearer($stranger)),
            404,
            'not_found',
        );
    }

    public function test_an_unverified_owner_cannot_grant_a_share(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1', false);
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        $this->assertError(
            $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'viewer'], $this->bearer($owner)),
            403,
            'email_not_verified',
        );
    }

    public function test_a_plan_without_sharing_refuses_a_grant(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'solo');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        $this->assertError(
            $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'viewer'], $this->bearer($owner)),
            403,
            'sharing_not_allowed',
        );
    }

    public function test_a_grant_with_an_invalid_access_or_a_self_share_is_refused(): void
    {
        [$owner, $friend, $instance] = $this->sharedWorld();

        $this->assertError(
            $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Friend_1', 'access' => 'owner'], $this->bearer($owner)),
            400,
            'invalid_access',
        );
        $this->assertError(
            $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Owner_1', 'access' => 'viewer'], $this->bearer($owner)),
            400,
            'invalid_share',
        );
        $this->assertError(
            $this->send('POST', "/api/instances/$instance/shares", ['username' => 'Nobody', 'access' => 'viewer'], $this->bearer($owner)),
            404,
            'not_found',
        );
    }

    public function test_removing_a_friendship_removes_the_shares_between_the_pair_in_both_directions(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        $third = ShareFixtures::user('third@example.test', 'Third_1');
        $friendship = ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::friendship($this->pdo(), $owner, $third);
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        ShareFixtures::plan($this->pdo(), $friend, 'unlimited');

        $ownersInstance = ShareFixtures::instance($this->pdo(), $owner);
        $friendsInstance = ShareFixtures::instance($this->pdo(), $friend, 'Friend world');
        $thirdsInstance = ShareFixtures::instance($this->pdo(), $owner, 'Third world');
        ShareFixtures::share($this->pdo(), $ownersInstance, $friend);
        ShareFixtures::share($this->pdo(), $friendsInstance, $owner, 'editor');
        ShareFixtures::share($this->pdo(), $thirdsInstance, $third);

        $removed = $this->send('DELETE', "/api/friends/$friendship", headers: $this->bearer($friend));

        $this->assertSame(204, $removed->getStatusCode());
        self::assertSame(0, ShareFixtures::shareCount($this->pdo(), $ownersInstance));
        self::assertSame(0, ShareFixtures::shareCount($this->pdo(), $friendsInstance));
        self::assertSame(1, ShareFixtures::shareCount($this->pdo(), $thirdsInstance));
    }

    /**
     * An owner with a friends plan, an accepted friend called Friend_1 and one instance. Returns the owner, the friend
     * and the instance ids.
     *
     * @return array{0: int, 1: int, 2: int}
     */
    private function sharedWorld(): array
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'friends');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        return [$owner, $friend, $instance];
    }

    /** Grants a share through the route and asserts it worked. */
    private function grantTo(int $owner, int $instance, string $username, string $access): void
    {
        $response = $this->send('POST', "/api/instances/$instance/shares", ['username' => $username, 'access' => $access], $this->bearer($owner));
        $this->assertContains($response->getStatusCode(), [200, 201]);
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
