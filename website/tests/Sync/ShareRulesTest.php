<?php

declare(strict_types=1);

namespace tests\Sync;

use app\Auth\AuthException;
use app\Friends\Friendships;
use app\Sync\Shares;
use PHPUnit\Framework\AssertionFailedError;
use tests\Support\DatabaseTestCase;

/**
 * The sharing rules in app\Sync\Shares: the cap, the plan, access values, self shares and friends only.
 */
final class ShareRulesTest extends DatabaseTestCase
{
    private Shares $shares;

    protected function setUp(): void
    {
        parent::setUp();

        $this->shares = new Shares();
    }

    public function test_a_friends_plan_caps_the_owner_at_five_shared_instances(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        $other = ShareFixtures::user('other@example.test', 'Other_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::friendship($this->pdo(), $owner, $other);
        ShareFixtures::plan($this->pdo(), $owner, 'friends');

        $instances = [];
        for ($i = 0; $i < 6; $i++) {
            $instances[] = ShareFixtures::instance($this->pdo(), $owner, 'World ' . $i);
        }

        for ($i = 0; $i < 5; $i++) {
            self::assertTrue($this->shares->grant($owner, $instances[$i], 'Friend_1', 'viewer')['created']);
        }

        $this->expectFailure('share_limit_reached', 409, fn () => $this->shares->grant($owner, $instances[5], 'Friend_1', 'viewer'));
    }

    public function test_a_second_friend_or_a_changed_access_on_a_shared_instance_does_not_count(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        $other = ShareFixtures::user('other@example.test', 'Other_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::friendship($this->pdo(), $owner, $other);
        ShareFixtures::plan($this->pdo(), $owner, 'friends');

        $instances = [];
        for ($i = 0; $i < 6; $i++) {
            $instances[] = ShareFixtures::instance($this->pdo(), $owner, 'World ' . $i);
        }
        for ($i = 0; $i < 5; $i++) {
            $this->shares->grant($owner, $instances[$i], 'Friend_1', 'viewer');
        }

        $second = $this->shares->grant($owner, $instances[0], 'Other_1', 'editor');
        self::assertTrue($second['created']);

        $changed = $this->shares->grant($owner, $instances[0], 'Friend_1', 'editor');
        self::assertFalse($changed['created']);
        self::assertSame('editor', $changed['share']['access']);
    }

    public function test_revoking_a_share_frees_a_slot_under_the_cap(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'friends');

        $instances = [];
        for ($i = 0; $i < 6; $i++) {
            $instances[] = ShareFixtures::instance($this->pdo(), $owner, 'World ' . $i);
        }
        for ($i = 0; $i < 5; $i++) {
            $this->shares->grant($owner, $instances[$i], 'Friend_1', 'viewer');
        }

        $this->shares->revoke($owner, $instances[0], 'Friend_1');

        self::assertTrue($this->shares->grant($owner, $instances[5], 'Friend_1', 'viewer')['created']);
    }

    public function test_the_cap_ignores_shares_on_other_owners_instances(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        $stranger = ShareFixtures::user('stranger@example.test', 'Stranger');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::friendship($this->pdo(), $stranger, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'friends');

        for ($i = 0; $i < 5; $i++) {
            ShareFixtures::share($this->pdo(), ShareFixtures::instance($this->pdo(), $stranger, 'Theirs ' . $i), $friend);
        }

        $mine = ShareFixtures::instance($this->pdo(), $owner, 'Mine');
        self::assertTrue($this->shares->grant($owner, $mine, 'Friend_1', 'viewer')['created']);
    }

    public function test_the_unlimited_plan_has_no_cap(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');

        for ($i = 0; $i < 8; $i++) {
            $instance = ShareFixtures::instance($this->pdo(), $owner, 'World ' . $i);
            self::assertTrue($this->shares->grant($owner, $instance, 'Friend_1', 'viewer')['created']);
        }
    }

    public function test_a_plan_without_sharing_refuses_a_grant(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        $this->expectFailure('sharing_not_allowed', 403, fn () => $this->shares->grant($owner, $instance, 'Friend_1', 'viewer'));

        ShareFixtures::plan($this->pdo(), $owner, 'solo');
        $this->expectFailure('sharing_not_allowed', 403, fn () => $this->shares->grant($owner, $instance, 'Friend_1', 'viewer'));

        self::assertSame(0, ShareFixtures::shareCount($this->pdo(), $instance));
    }

    public function test_only_viewer_and_editor_are_access_values(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        foreach (['owner', 'Viewer', 'EDITOR', '', null, 1] as $access) {
            $this->expectFailure('invalid_access', 400, fn () => $this->shares->grant($owner, $instance, 'Friend_1', $access));
        }

        self::assertSame('editor', $this->shares->grant($owner, $instance, 'Friend_1', 'editor')['share']['access']);
    }

    public function test_an_owner_cannot_share_with_themselves(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        $this->expectFailure('invalid_share', 400, fn () => $this->shares->grant($owner, $instance, 'owner_1', 'viewer'));
    }

    public function test_a_user_who_is_not_a_friend_cannot_be_granted_a_share(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $pending = ShareFixtures::user('pending@example.test', 'Pending_1');
        ShareFixtures::user('stranger@example.test', 'Stranger');
        ShareFixtures::friendship($this->pdo(), $owner, $pending, 'pending');
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        $this->expectFailure('not_found', 404, fn () => $this->shares->grant($owner, $instance, 'Pending_1', 'viewer'));
        $this->expectFailure('not_found', 404, fn () => $this->shares->grant($owner, $instance, 'Stranger', 'viewer'));
        $this->expectFailure('not_found', 404, fn () => $this->shares->grant($owner, $instance, 'Nobody', 'viewer'));
        self::assertSame(0, ShareFixtures::shareCount($this->pdo(), $instance));
    }

    public function test_a_friendship_started_by_the_friend_counts(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1');
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $friend, $owner);
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        self::assertSame('viewer', $this->shares->grant($owner, $instance, 'Friend_1', 'viewer')['share']['access']);
    }

    public function test_an_unverified_owner_cannot_grant(): void
    {
        $owner = ShareFixtures::user('owner@example.test', 'Owner_1', false);
        $friend = ShareFixtures::user('friend@example.test', 'Friend_1');
        ShareFixtures::friendship($this->pdo(), $owner, $friend);
        ShareFixtures::plan($this->pdo(), $owner, 'unlimited');
        $instance = ShareFixtures::instance($this->pdo(), $owner);

        $this->expectFailure('email_not_verified', 403, fn () => $this->shares->grant($owner, $instance, 'Friend_1', 'viewer'));
    }

    public function test_removing_a_friendship_drops_shares_in_both_directions(): void
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

        (new Friendships())->remove($owner, $friendship);

        self::assertSame(0, ShareFixtures::shareCount($this->pdo(), $ownersInstance));
        self::assertSame(0, ShareFixtures::shareCount($this->pdo(), $friendsInstance));
        self::assertSame(1, ShareFixtures::shareCount($this->pdo(), $thirdsInstance));
    }

    /** Runs the work and checks it throws the expected error code and status. */
    private function expectFailure(string $code, int $status, callable $work): void
    {
        try {
            $work();
        } catch (AuthException $error) {
            self::assertSame($code, $error->errorCode);
            self::assertSame($status, $error->httpStatus);

            return;
        }

        throw new AssertionFailedError("Expected $code ($status) but the call succeeded");
    }
}
