<?php

declare(strict_types=1);

namespace tests\Minecraft;

use app\Auth\AuthException;
use app\Minecraft\LinkService;
use tests\Auth\OAuth\FakeTransport;
use tests\Support\DatabaseTestCase;

/**
 * The link flow end to end through LinkService, with every Microsoft, Xbox and Minecraft call answered by the fake.
 */
final class MinecraftLinkTest extends DatabaseTestCase
{
    use LinkFixtures;

    private const ISO_PATTERN = '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00$/';

    private LinkService $service;
    private int $userId;

    protected function setUp(): void
    {
        parent::setUp();

        $this->installTransport();
        $this->service = new LinkService($this->transport);
        $this->userId = $this->createUser('steve@example.test');
    }

    protected function tearDown(): void
    {
        $this->restoreTransport();

        parent::tearDown();
    }

    public function testStartRequestsADeviceCodeForTheLauncherAppAndReturnsTheCodeToEnter(): void
    {
        $this->answerDeviceCode();

        $started = $this->service->start($this->userId);

        self::assertSame(
            ['client_id' => self::MS_CLIENT_ID, 'scope' => 'XboxLive.signin offline_access'],
            $this->lastRequestTo(self::DEVICE_URL)['form'],
        );
        self::assertSame(32, strlen($started['id']));
        self::assertSame(self::USER_CODE, $started['user_code']);
        self::assertSame('https://www.microsoft.com/link', $started['verification_uri']);
        self::assertSame(5, $started['interval']);
        self::assertMatchesRegularExpression(self::ISO_PATTERN, $started['expires_at']);
        self::assertArrayNotHasKey('device_code', $started);
    }

    public function testStartingAgainDeletesTheEarlierPendingSession(): void
    {
        $first = $this->startLink($this->service, $this->userId);
        $second = $this->startLink($this->service, $this->userId);

        self::assertNotSame($first, $second);
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$first]));
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$second]));
    }

    public function testAMicrosoftOutageAtStartIsUnavailable(): void
    {
        $this->transport->respondWith(self::DEVICE_URL, FakeTransport::json(['error' => 'server_error'], 503));

        $this->assertAuthError(
            fn () => $this->service->start($this->userId),
            'minecraft_unavailable',
            502,
        );
    }

    public function testAPendingPollReturnsPendingWithTheInterval(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'authorization_pending']);

        self::assertSame(['status' => 'pending', 'interval' => 5], $this->service->status($this->userId, $id));

        self::assertSame(
            [
                'client_id' => self::MS_CLIENT_ID,
                'grant_type' => 'urn:ietf:params:oauth:grant-type:device_code',
                'device_code' => self::DEVICE_CODE,
            ],
            $this->lastRequestTo(self::TOKEN_URL)['form'],
        );
    }

    public function testSlowDownAddsFiveSecondsAndAPollInsideTheIntervalIsRefused(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'slow_down']);

        self::assertSame(['status' => 'pending', 'interval' => 10], $this->service->status($this->userId, $id));
        self::assertSame(
            10,
            $this->scalar('SELECT interval_seconds FROM minecraft_link_sessions WHERE id = ?', [$id]),
        );

        $this->assertAuthError(
            fn () => $this->service->status($this->userId, $id),
            'slow_down',
            429,
            retryAfter: 10,
        );
    }

    public function testADeclinedSignInIsDeniedAndTheSessionEnds(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'authorization_declined']);

        self::assertSame(['status' => 'denied'], $this->service->status($this->userId, $id));
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    public function testAnExpiredCodeFromMicrosoftEndsTheSession(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'expired_token']);

        self::assertSame(['status' => 'expired'], $this->service->status($this->userId, $id));
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    public function testASessionPastItsExpiryEndsWithoutPollingMicrosoft(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->pdo()->prepare(
            'UPDATE minecraft_link_sessions SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE id = ?'
        )->execute([$id]);

        self::assertSame(['status' => 'expired'], $this->service->status($this->userId, $id));
        self::assertSame([], $this->requestsTo(self::TOKEN_URL));
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    public function testAnOwnedAccountIsLinkedAndTheChainUsesTheMicrosoftTokenOnce(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();

        $result = $this->service->status($this->userId, $id);

        self::assertSame('linked', $result['status']);
        self::assertSame(self::UUID, $result['account']['uuid']);
        self::assertSame(self::NAME, $result['account']['name']);
        self::assertMatchesRegularExpression(self::ISO_PATTERN, $result['account']['linked_at']);

        self::assertSame(
            [['uuid' => self::UUID, 'user_id' => $this->userId, 'name' => self::NAME]],
            array_map(
                static fn (array $row): array => ['uuid' => $row['uuid'], 'user_id' => (int) $row['user_id'], 'name' => $row['name']],
                $this->rows('SELECT uuid, user_id, name FROM minecraft_accounts'),
            ),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));

        self::assertSame('d=' . self::MICROSOFT_TOKEN, $this->lastRequestTo(self::XBOX_USER_URL)['json']['Properties']['RpsTicket']);
        self::assertSame(self::XBOX_TOKEN, $this->lastRequestTo(self::XSTS_URL)['json']['Properties']['UserTokens'][0]);
        self::assertSame(
            'XBL3.0 x=' . self::USER_HASH . ';' . self::XSTS_TOKEN,
            $this->lastRequestTo(self::MINECRAFT_LOGIN_URL)['json']['identityToken'],
        );
        self::assertSame('Bearer ' . self::MINECRAFT_TOKEN, $this->lastRequestTo(self::PROFILE_URL)['headers']['Authorization']);
    }

    public function testAnAccountWithoutAMinecraftProfileIsNotOwned(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::PROFILE_URL, FakeTransport::json(['error' => 'NOT_FOUND'], 404));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_not_owned'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    public function testAnAccountWithNoXboxAccountIsNotOwned(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['XErr' => 2148916233], 401));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_not_owned'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    public function testAnAccountAlreadyHeldByAnotherUserIsLinkedElsewhere(): void
    {
        $other = $this->createUser('alex@example.test');
        $this->pdo()->prepare('INSERT INTO minecraft_accounts (uuid, user_id, name) VALUES (?, ?, ?)')
            ->execute([self::UUID, $other, 'Alex']);

        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_linked_elsewhere'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(
            [['user_id' => $other, 'name' => 'Alex']],
            array_map(
                static fn (array $row): array => ['user_id' => (int) $row['user_id'], 'name' => $row['name']],
                $this->rows('SELECT user_id, name FROM minecraft_accounts'),
            ),
        );
    }

    public function testAChildAccountNotInAFamilyIsRestricted(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['XErr' => 2148916238], 401));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_restricted'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    public function testACountryAgeOrRegionRestrictionEndsTheSessionAsRestricted(): void
    {
        // DESIGN.md section 6: 2148916234 not available in the country, 2148916235 region unsupported,
        // 2148916236 and 2148916237 adult verification required.
        foreach ([2148916234, 2148916235, 2148916236, 2148916237] as $code) {
            $id = $this->startLink($this->service, $this->userId);
            $this->answerOwnedChain();
            $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['XErr' => $code], 401));

            self::assertSame(
                ['status' => 'failed', 'error' => 'minecraft_restricted'],
                $this->service->status($this->userId, $id),
                'XErr ' . $code,
            );
            self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$id]));
        }
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    public function testAnXboxCodeTheTableDoesNotNameEndsTheSessionAsUnavailable(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['XErr' => 2148916299], 401));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_unavailable'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    public function testAProfileNameLongerThanSixteenCharactersIsUnavailableAndNotStored(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::PROFILE_URL, FakeTransport::json([
            'id' => self::UUID_HEX,
            'name' => 'ABCDEFGHIJKLMNOPQ',
        ]));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_unavailable'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    public function testAProfileUuidThatIsNotThirtyTwoHexCharactersIsUnavailableAndNotStored(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::PROFILE_URL, FakeTransport::json([
            'id' => '0123abcd0123abcd0123abcd0123abcx',
            'name' => self::NAME,
        ]));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_unavailable'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    public function testAnXboxServiceFailureEndsTheSessionAsUnavailable(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['error' => 'server_error'], 500));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_unavailable'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions'));
    }

    public function testAnUnreadableMicrosoftPollIsUnavailableAndKeepsTheSession(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->transport->respondWith(self::TOKEN_URL, ['status' => 500, 'body' => '']);

        $this->assertAuthError(
            fn () => $this->service->status($this->userId, $id),
            'minecraft_unavailable',
            502,
        );
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$id]));
    }

    public function testRelinkingAfterARenameUpdatesTheNameAndKeepsOneRow(): void
    {
        $first = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->service->status($this->userId, $first);

        $second = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::PROFILE_URL, FakeTransport::json([
            'id' => self::UUID_HEX,
            'name' => 'SteveRenamed',
        ]));
        $result = $this->service->status($this->userId, $second);

        self::assertSame('linked', $result['status']);
        self::assertSame('SteveRenamed', $result['account']['name']);
        self::assertSame(
            [['uuid' => self::UUID, 'user_id' => $this->userId, 'name' => 'SteveRenamed']],
            array_map(
                static fn (array $row): array => ['uuid' => $row['uuid'], 'user_id' => (int) $row['user_id'], 'name' => $row['name']],
                $this->rows('SELECT uuid, user_id, name FROM minecraft_accounts'),
            ),
        );
    }

    public function testRelinkingTheSameAccountKeepsOneRow(): void
    {
        $first = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->service->status($this->userId, $first);

        $second = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $result = $this->service->status($this->userId, $second);

        self::assertSame('linked', $result['status']);
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts WHERE user_id = ?', [$this->userId]));
        self::assertCount(1, $this->service->accounts($this->userId)['accounts']);
    }

    public function testAnXboxRefusalIsReadWhateverTheHttpStatus(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['XErr' => 2148916233], 403));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_not_owned'],
            $this->service->status($this->userId, $id),
        );
    }

    public function testAnXErrSentAsANumericStringIsMappedLikeTheNumber(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerOwnedChain();
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json(['XErr' => '2148916238'], 401));

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_restricted'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    public function testAnUnknownClientErrorIsUnavailableAndKeepsTheSessionForTheNextPoll(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'unsupported_grant_type'], 400);

        $this->assertAuthError(
            fn () => $this->service->status($this->userId, $id),
            'minecraft_unavailable',
            502,
        );
        self::assertSame(1, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$id]));
        self::assertCount(1, $this->requestsTo(self::TOKEN_URL));

        // The next poll is allowed once the interval has passed, and it can still succeed.
        $this->pdo()->prepare(
            'UPDATE minecraft_link_sessions SET last_polled_at = DATE_SUB(NOW(6), INTERVAL 60 SECOND) WHERE id = ?'
        )->execute([$id]);
        $this->answerPoll(['error' => 'authorization_pending']);

        self::assertSame(['status' => 'pending', 'interval' => 5], $this->service->status($this->userId, $id));
    }

    public function testInvalidGrantFinishesTheSessionAsUnavailable(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'invalid_grant'], 400);

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_unavailable'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$id]));
        self::assertCount(1, $this->requestsTo(self::TOKEN_URL));
    }

    public function testBadVerificationCodeFinishesTheSessionAsUnavailable(): void
    {
        $id = $this->startLink($this->service, $this->userId);
        $this->answerPoll(['error' => 'bad_verification_code'], 400);

        self::assertSame(
            ['status' => 'failed', 'error' => 'minecraft_unavailable'],
            $this->service->status($this->userId, $id),
        );
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE id = ?', [$id]));
        self::assertCount(1, $this->requestsTo(self::TOKEN_URL));
    }

    public function testAProfileNameOutsideTheMinecraftCharacterSetIsUnavailableAndNotStored(): void
    {
        foreach (['Ste ve', 'Steve-1', 'Stéve'] as $name) {
            $id = $this->startLink($this->service, $this->userId);
            $this->answerOwnedChain();
            $this->transport->respondWith(self::PROFILE_URL, FakeTransport::json([
                'id' => self::UUID_HEX,
                'name' => $name,
            ]));

            self::assertSame(
                ['status' => 'failed', 'error' => 'minecraft_unavailable'],
                $this->service->status($this->userId, $id),
                'name ' . $name,
            );
        }

        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM minecraft_accounts'));
    }

    private function assertAuthError(
        callable $call,
        string $code,
        int $status,
        ?int $retryAfter = null,
    ): void {
        try {
            $call();
            self::fail('The call succeeded, but ' . $code . ' was expected');
        } catch (AuthException $error) {
            self::assertSame($code, $error->errorCode);
            self::assertSame($status, $error->httpStatus);
            self::assertSame($retryAfter, $error->retryAfter);
        }
    }
}
