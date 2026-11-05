<?php

declare(strict_types=1);

namespace tests\Friends;

use app\Auth\AuthException;
use app\Auth\Tokens;
use app\Friends\FriendCodes;
use app\Friends\FriendInvites;
use app\Friends\Friendships;
use app\Users\UserRepository;
use PDO;
use tests\Support\DatabaseTestCase;

/**
 * Friend requests, accept and remove, friend codes and invite links. The route checks (verified email, username,
 * rate limits) are tested in the HTTP suite.
 */
final class FriendshipTest extends DatabaseTestCase
{
    private Friendships $friendships;
    private FriendCodes $codes;
    private FriendInvites $invites;
    private UserRepository $users;

    private int $ada;
    private int $grace;
    private int $linus;

    protected function setUp(): void
    {
        parent::setUp();

        $this->friendships = new Friendships();
        $this->codes = new FriendCodes();
        $this->invites = new FriendInvites();
        $this->users = new UserRepository();

        $this->ada = $this->member('ada@example.test', 'Ada_L');
        $this->grace = $this->member('grace@example.test', 'Grace');
        $this->linus = $this->member('linus@example.test', 'linus');
    }

    public function test_a_request_is_pending_until_the_addressee_accepts(): void
    {
        $request = $this->friendships->request($this->ada, $this->grace);

        self::assertSame('pending', $request['status']);
        self::assertSame('Grace', $request['username']);
        self::assertSame(['friends' => [], 'incoming' => [], 'outgoing' => [
            ['id' => $request['id'], 'username' => 'Grace'],
        ]], $this->list($this->ada));

        $this->friendships->accept($this->grace, $request['id']);

        self::assertSame(['friends' => [['id' => $request['id'], 'username' => 'Grace']], 'incoming' => [], 'outgoing' => []], $this->list($this->ada));
        self::assertSame(['friends' => [['id' => $request['id'], 'username' => 'Ada_L']], 'incoming' => [], 'outgoing' => []], $this->list($this->grace));
    }

    public function test_a_request_to_yourself_is_invalid(): void
    {
        $this->assertError('invalid_friend', fn () => $this->friendships->request($this->ada, $this->ada));
    }

    public function test_a_request_to_an_unknown_or_deleted_account_is_not_found(): void
    {
        $this->assertError('not_found', fn () => $this->friendships->request($this->ada, 999999));

        $this->pdo()->prepare('UPDATE users SET deleted_at = NOW(6) WHERE id = ?')->execute([$this->linus]);

        $this->assertError('not_found', fn () => $this->friendships->request($this->ada, $this->linus));
    }

    public function test_a_second_request_in_the_same_direction_is_pending(): void
    {
        $this->friendships->request($this->ada, $this->grace);

        $this->assertError('request_pending', fn () => $this->friendships->request($this->ada, $this->grace));
    }

    public function test_a_request_back_from_the_target_accepts_the_pending_one(): void
    {
        $first = $this->friendships->request($this->ada, $this->grace);

        $back = $this->friendships->request($this->grace, $this->ada);

        self::assertSame(['status' => 'accepted', 'friend' => ['username' => 'Ada_L']], $back);
        self::assertCount(1, $this->list($this->ada)['friends']);
        self::assertSame($first['id'], $this->list($this->ada)['friends'][0]['id']);
        self::assertSame(1, $this->rowCount());
    }

    public function test_a_request_between_friends_reports_already_friends_in_either_direction(): void
    {
        $this->befriend($this->ada, $this->grace);

        $this->assertError('already_friends', fn () => $this->friendships->request($this->ada, $this->grace));
        $this->assertError('already_friends', fn () => $this->friendships->request($this->grace, $this->ada));
    }

    public function test_only_the_addressee_can_accept_a_request(): void
    {
        $request = $this->friendships->request($this->ada, $this->grace);

        $this->assertError('not_found', fn () => $this->friendships->accept($this->ada, $request['id']));
        $this->assertError('not_found', fn () => $this->friendships->accept($this->linus, $request['id']));
    }

    public function test_either_party_can_end_a_friendship_and_a_stranger_cannot(): void
    {
        $id = $this->befriend($this->ada, $this->grace);

        $this->assertError('not_found', fn () => $this->friendships->remove($this->linus, $id));

        $this->friendships->remove($this->grace, $id);

        self::assertSame(['friends' => [], 'incoming' => [], 'outgoing' => []], $this->list($this->ada));
    }

    public function test_either_party_can_cancel_or_decline_a_pending_request(): void
    {
        $request = $this->friendships->request($this->ada, $this->grace);

        $this->friendships->remove($this->grace, $request['id']);

        self::assertSame(0, $this->rowCount());
        $this->assertError('not_found', fn () => $this->friendships->remove($this->ada, $request['id']));
    }

    public function test_the_lists_carry_usernames_and_ids_and_no_email_or_account_ids(): void
    {
        $this->friendships->request($this->ada, $this->grace);

        $lists = $this->list($this->grace);

        self::assertSame(['friends', 'incoming', 'outgoing'], array_keys($lists));
        self::assertSame(['id', 'username'], array_keys($lists['incoming'][0]));
        self::assertSame('Ada_L', $lists['incoming'][0]['username']);
        self::assertStringNotContainsString('ada@example.test', json_encode($lists, JSON_THROW_ON_ERROR));
    }

    public function test_a_friend_code_is_created_once_and_reset_replaces_it(): void
    {
        $code = $this->codes->forUser($this->ada);

        self::assertMatchesRegularExpression('/^[ABCDEFGHJKMNPQRSTVWXYZ23456789]{8}$/', $code);
        self::assertSame($code, $this->codes->forUser($this->ada));
        self::assertSame($this->ada, $this->codes->ownerId($code));

        $new = $this->codes->reset($this->ada);

        self::assertNotSame($code, $new);
        self::assertNull($this->codes->ownerId($code));
        self::assertSame($this->ada, $this->codes->ownerId($new));
    }

    public function test_a_friend_code_can_send_a_request_to_its_owner(): void
    {
        $code = $this->codes->forUser($this->grace);

        $request = $this->friendships->request($this->ada, (int) $this->codes->ownerId($code));

        self::assertSame('pending', $request['status']);
    }

    public function test_an_invite_befriends_the_inviter_once_and_only_once(): void
    {
        $invite = $this->invites->create($this->grace);

        self::assertSame(43, strlen($invite['token']));
        self::assertMatchesRegularExpression('/^\d{4}-\d{2}-\d{2}T/', $invite['expires_at']);
        self::assertSame($this->expiryWindow($invite['token']), 604800);

        $result = $this->invites->accept($this->ada, $invite['token']);

        self::assertSame(['status' => 'accepted', 'friend' => ['username' => 'Grace']], $result);
        self::assertCount(1, $this->list($this->ada)['friends']);

        $this->assertError('invite_used', fn () => $this->invites->accept($this->linus, $invite['token']));
    }

    public function test_an_expired_invite_is_gone(): void
    {
        $invite = $this->invites->create($this->grace);
        $this->pdo()->prepare(
            'UPDATE friend_invites SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE token_hash = ?'
        )->execute([Tokens::hash($invite['token'])]);

        $this->assertError('invite_expired', fn () => $this->invites->accept($this->ada, $invite['token']));
    }

    public function test_an_unknown_invite_is_not_found_and_your_own_invite_is_invalid(): void
    {
        $this->assertError('invalid_invite', fn () => $this->invites->accept($this->ada, 'unknown-token'));

        $own = $this->invites->create($this->ada);
        $this->assertError('invalid_friend', fn () => $this->invites->accept($this->ada, $own['token']));
    }

    public function test_an_invite_to_someone_already_a_friend_reports_already_friends(): void
    {
        $this->befriend($this->ada, $this->grace);
        $invite = $this->invites->create($this->grace);

        $this->assertError('already_friends', fn () => $this->invites->accept($this->ada, $invite['token']));
    }

    public function test_an_invite_turns_a_pending_request_into_a_friendship(): void
    {
        $request = $this->friendships->request($this->ada, $this->grace);
        $invite = $this->invites->create($this->grace);

        $this->invites->accept($this->ada, $invite['token']);

        self::assertSame(1, $this->rowCount());
        self::assertSame([['id' => $request['id'], 'username' => 'Grace']], $this->list($this->ada)['friends']);
    }

    private function member(string $email, string $username): int
    {
        $id = $this->users->create($email, null, null);
        $this->users->setUsername($id, $username);

        return $id;
    }

    /** Makes the pair friends through a request and its acceptance, and returns the friendship id. */
    private function befriend(int $requester, int $addressee): int
    {
        $request = $this->friendships->request($requester, $addressee);
        $this->friendships->accept($addressee, $request['id']);

        return $request['id'];
    }

    /** @return array{friends: list<array{id: int, username: ?string}>, incoming: list<array{id: int, username: ?string}>, outgoing: list<array{id: int, username: ?string}>} */
    private function list(int $userId): array
    {
        return $this->friendships->list($userId);
    }

    private function rowCount(): int
    {
        return (int) $this->pdo()->query('SELECT COUNT(*) FROM friendships')->fetchColumn();
    }

    /** The seconds between creation and expiry of the invite with this token. */
    private function expiryWindow(string $token): int
    {
        $statement = $this->pdo()->prepare(
            'SELECT TIMESTAMPDIFF(SECOND, created_at, expires_at) FROM friend_invites WHERE token_hash = ?'
        );
        $statement->execute([Tokens::hash($token)]);

        return (int) $statement->fetchColumn();
    }

    private function assertError(string $code, callable $call): void
    {
        try {
            $call();
        } catch (AuthException $error) {
            self::assertSame($code, $error->errorCode);

            return;
        }

        self::fail("Expected the error $code");
    }
}
