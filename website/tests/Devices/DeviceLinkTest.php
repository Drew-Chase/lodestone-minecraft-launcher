<?php

declare(strict_types=1);

namespace tests\Devices;

use app\Auth\AuthException;
use app\Auth\RefreshTokens;
use app\Auth\Tokens;
use app\Devices\DeviceLinks;
use app\Devices\Devices;
use app\Devices\DeviceTokens;
use app\Users\UserRepository;
use PDO;
use tests\Support\DatabaseTestCase;

/**
 * The device sign-in service: start, approve and the token exchange, with its pending, slow down, limit and reuse rules.
 */
final class DeviceLinkTest extends DatabaseTestCase
{
    private const IP = '198.51.100.50';
    private const USER_CODE_PATTERN = '/^[ABCDEFGHJKMNPQRSTVWXYZ23456789]{4}-[ABCDEFGHJKMNPQRSTVWXYZ23456789]{4}$/';
    private const FIELDS = ['name' => 'Drew PC', 'os' => 'Windows 11', 'app_version' => '1.4.0'];

    private DeviceLinks $links;
    private DeviceTokens $tokens;
    private int $userId;

    protected function setUp(): void
    {
        parent::setUp();

        $this->links = new DeviceLinks();
        $this->tokens = new DeviceTokens();
        $this->userId = (new UserRepository())->create('linker@example.test', null, null);
    }

    public function test_start_returns_a_device_code_and_a_hyphenated_user_code(): void
    {
        $started = $this->links->start(self::FIELDS);

        self::assertMatchesRegularExpression('/^[A-Za-z0-9_-]{43}$/', $started['device_code']);
        self::assertMatchesRegularExpression(self::USER_CODE_PATTERN, $started['user_code']);

        $row = $this->link($started['device_code']);
        self::assertSame('pending', $row['status']);
        self::assertSame(str_replace('-', '', $started['user_code']), $row['user_code']);
    }

    public function test_only_the_hash_of_the_device_code_is_stored(): void
    {
        $started = $this->links->start(self::FIELDS);

        $stored = $this->pdo()->query('SELECT device_code_hash FROM device_links')->fetchAll(PDO::FETCH_COLUMN);

        self::assertSame([Tokens::hash($started['device_code'])], $stored);
    }

    public function test_polling_before_approval_is_pending_and_a_fast_repeat_is_slowed(): void
    {
        $started = $this->links->start(self::FIELDS);

        $this->assertError('authorization_pending', fn () => $this->tokens->redeem($started['device_code'], self::IP));
        $this->assertError('slow_down', fn () => $this->tokens->redeem($started['device_code'], self::IP), 5);

        $this->backdatePoll($started['device_code'], 6);

        $this->assertError('authorization_pending', fn () => $this->tokens->redeem($started['device_code'], self::IP));
    }

    public function test_approve_accepts_the_code_without_its_hyphen_and_in_lower_case(): void
    {
        $started = $this->links->start(self::FIELDS);

        $this->links->approve($this->userId, strtolower(str_replace('-', '', $started['user_code'])));

        $row = $this->link($started['device_code']);
        self::assertSame('approved', $row['status']);
        self::assertSame($this->userId, (int) $row['user_id']);
        self::assertNotNull($row['approved_at']);
    }

    public function test_approve_rejects_an_unknown_or_malformed_code(): void
    {
        $this->assertError('invalid_code', fn () => $this->links->approve($this->userId, 'AAAA-AAAA'));
        $this->assertError('invalid_code', fn () => $this->links->approve($this->userId, 'nope'));
    }

    public function test_approve_of_an_expired_code_reports_code_expired_and_leaves_it_pending(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->expire($started['device_code']);

        $this->assertError('code_expired', fn () => $this->links->approve($this->userId, $started['user_code']));
        self::assertSame('pending', $this->link($started['device_code'])['status']);
    }

    public function test_approve_of_a_code_already_approved_reports_code_used(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->links->approve($this->userId, $started['user_code']);

        $this->assertError('code_used', fn () => $this->links->approve($this->userId, $started['user_code']));
    }

    public function test_approve_refuses_at_the_device_limit_and_leaves_the_code_pending(): void
    {
        // The free plan allows one device.
        $this->device($this->userId);
        $started = $this->links->start(self::FIELDS);

        $this->assertError('device_limit_reached', fn () => $this->links->approve($this->userId, $started['user_code']));
        self::assertSame('pending', $this->link($started['device_code'])['status']);

        $this->pdo()->prepare('UPDATE devices SET revoked_at = NOW(6) WHERE user_id = ?')->execute([$this->userId]);

        $this->links->approve($this->userId, $started['user_code']);
        self::assertSame('approved', $this->link($started['device_code'])['status']);
    }

    public function test_redeem_checks_the_limit_again_and_keeps_the_approval_when_it_is_reached(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->links->approve($this->userId, $started['user_code']);
        $this->device($this->userId);

        $this->assertError('device_limit_reached', fn () => $this->tokens->redeem($started['device_code'], self::IP));

        self::assertSame('approved', $this->link($started['device_code'])['status']);
        self::assertSame(1, $this->deviceCount($this->userId));
    }

    public function test_redeem_issues_a_launcher_session_and_consumes_the_approval(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->links->approve($this->userId, $started['user_code']);

        $session = $this->tokens->redeem($started['device_code'], self::IP);

        self::assertNotSame('', $session['access_token']);
        self::assertSame(900, $session['expires_in']);
        self::assertMatchesRegularExpression('/^[a-f0-9]{64}$/', $session['refresh_token']);
        self::assertSame($this->userId, $session['user']['id']);
        self::assertSame('Drew PC', $session['device']['name']);

        $row = $this->link($started['device_code']);
        self::assertSame('consumed', $row['status']);
        self::assertSame($session['device']['id'], (int) $row['device_id']);
        self::assertNotNull($row['consumed_at']);

        $issued = $this->pdo()->prepare('SELECT client, device_id FROM refresh_tokens WHERE token_hash = ?');
        $issued->execute([Tokens::hash($session['refresh_token'])]);
        $token = $issued->fetch(PDO::FETCH_ASSOC);
        self::assertSame('launcher', $token['client']);
        self::assertSame($session['device']['id'], (int) $token['device_id']);
    }

    public function test_a_consumed_code_cannot_be_redeemed_or_approved_again(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->links->approve($this->userId, $started['user_code']);
        $this->tokens->redeem($started['device_code'], self::IP);

        $this->assertError('expired_token', fn () => $this->tokens->redeem($started['device_code'], self::IP));
        $this->assertError('code_used', fn () => $this->links->approve($this->userId, $started['user_code']));
    }

    public function test_an_approval_stays_redeemable_after_the_code_expires(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->links->approve($this->userId, $started['user_code']);
        $this->expire($started['device_code']);

        $session = $this->tokens->redeem($started['device_code'], self::IP);

        self::assertNotSame('', $session['access_token']);
    }

    public function test_a_pending_code_past_its_expiry_reports_expired_token(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->expire($started['device_code']);

        $this->assertError('expired_token', fn () => $this->tokens->redeem($started['device_code'], self::IP));
    }

    public function test_a_denied_sign_in_reports_access_denied(): void
    {
        $started = $this->links->start(self::FIELDS);
        $this->pdo()->prepare("UPDATE device_links SET status = 'denied' WHERE device_code_hash = ?")
            ->execute([Tokens::hash($started['device_code'])]);

        $this->assertError('access_denied', fn () => $this->tokens->redeem($started['device_code'], self::IP));
    }

    public function test_an_unknown_device_code_is_invalid(): void
    {
        $this->assertError('invalid_device_code', fn () => $this->tokens->redeem('not-a-real-device-code', self::IP));
    }

    /** @return array<string, mixed> */
    public function test_revoking_a_device_ends_its_family_and_leaves_another_device_live(): void
    {
        $revoked = $this->device($this->userId);
        $kept = $this->device($this->userId);
        $ended = (new RefreshTokens())->issue($this->userId, 'launcher', $revoked);
        $live = (new RefreshTokens())->issue($this->userId, 'launcher', $kept);

        (new Devices())->revoke($this->userId, $revoked);

        $active = $this->pdo()->prepare('SELECT id FROM devices WHERE user_id = ? AND revoked_at IS NULL');
        $active->execute([$this->userId]);
        self::assertSame([$kept], array_map('intval', $active->fetchAll(PDO::FETCH_COLUMN)));
        $this->assertError('refresh_invalid', fn () => (new RefreshTokens())->rotate($ended['token'], 'launcher', self::IP));
        self::assertSame(
            $this->userId,
            (new RefreshTokens())->rotate($live['token'], 'launcher', self::IP)['user_id'],
        );
    }

    public function test_revoking_another_users_device_is_not_found_and_changes_nothing(): void
    {
        $stranger = (new UserRepository())->create('stranger@example.test', null, null);
        $device = $this->device($stranger);
        $issued = (new RefreshTokens())->issue($stranger, 'launcher', $device);

        $this->assertError('not_found', fn () => (new Devices())->revoke($this->userId, $device));

        self::assertSame(1, $this->deviceCount($stranger));
        self::assertSame(
            $stranger,
            (new RefreshTokens())->rotate($issued['token'], 'launcher', self::IP)['user_id'],
        );
    }

    private function link(string $deviceCode): array
    {
        $statement = $this->pdo()->prepare('SELECT * FROM device_links WHERE device_code_hash = ?');
        $statement->execute([Tokens::hash($deviceCode)]);
        $row = $statement->fetch(PDO::FETCH_ASSOC);
        self::assertIsArray($row);

        return $row;
    }

    private function expire(string $deviceCode): void
    {
        $this->pdo()->prepare(
            'UPDATE device_links SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE device_code_hash = ?'
        )->execute([Tokens::hash($deviceCode)]);
    }

    private function backdatePoll(string $deviceCode, int $seconds): void
    {
        $this->pdo()->prepare(
            'UPDATE device_links SET last_polled_at = DATE_SUB(NOW(6), INTERVAL ' . $seconds . ' SECOND)
             WHERE device_code_hash = ?'
        )->execute([Tokens::hash($deviceCode)]);
    }

    private function device(int $userId): int
    {
        return (new Devices())->add($userId, 'Old PC', 'Windows 10', '1.0.0');
    }

    private function deviceCount(int $userId): int
    {
        $statement = $this->pdo()->prepare('SELECT COUNT(*) FROM devices WHERE user_id = ?');
        $statement->execute([$userId]);

        return (int) $statement->fetchColumn();
    }

    private function assertError(string $code, callable $call, ?int $retryAfter = null): void
    {
        try {
            $call();
        } catch (AuthException $error) {
            self::assertSame($code, $error->errorCode);
            if ($retryAfter !== null) {
                self::assertSame($retryAfter, $error->retryAfter);
            }

            return;
        }

        self::fail("Expected the error $code");
    }
}
