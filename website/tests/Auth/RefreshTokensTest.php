<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\AuthException;
use app\Auth\RefreshTokens;
use app\Auth\Tokens;
use app\Db\Database;
use app\Devices\Devices;
use app\Users\UserRepository;
use tests\Support\DatabaseTestCase;

final class RefreshTokensTest extends DatabaseTestCase
{
    private RefreshTokens $refresh;
    private int $userId;

    protected function setUp(): void
    {
        parent::setUp();

        $this->refresh = new RefreshTokens();
        $this->userId = (new UserRepository())->create('refresh@example.test', null, null);
    }

    public function testRotateReturnsANewTokenInTheSameFamily(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');

        $rotated = $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');

        self::assertSame($this->userId, $rotated['user_id']);
        self::assertSame($issued['family_id'], $rotated['family_id']);
        self::assertNotSame($issued['token'], $rotated['token']);
        self::assertTrue(Tokens::isWellFormed($rotated['token']));
    }

    public function testOldTokenIsMarkedUsedAfterRotation(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');
        $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');

        $usedAt = $this->pdo()->prepare('SELECT used_at FROM refresh_tokens WHERE token_hash = ?');
        $usedAt->execute([Tokens::hash($issued['token'])]);

        self::assertNotNull($usedAt->fetchColumn());
    }

    public function testRotatingAUsedTokenIsReuseAndRevokesTheFamily(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');
        $newer = $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');

        // Past the grace window a used token is reuse. Backdate its rotation time to get there.
        $backdate = $this->pdo()->prepare(
            'UPDATE refresh_tokens SET used_at = DATE_SUB(NOW(6), INTERVAL 11 SECOND) WHERE token_hash = ?'
        );
        $backdate->execute([Tokens::hash($issued['token'])]);

        try {
            $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');
            self::fail('Rotating a used token must throw refresh_reused.');
        } catch (AuthException $error) {
            self::assertSame('refresh_reused', $error->errorCode);
        }

        // The revocation is committed, so the replacement token no longer works.
        try {
            $this->refresh->rotate($newer['token'], 'web', '127.0.0.1');
            self::fail('The family must be revoked after reuse.');
        } catch (AuthException $error) {
            self::assertSame('refresh_invalid', $error->errorCode);
        }
    }

    public function testRotatingAUsedTokenWithinTheGraceWindowIsInvalidAndKeepsTheFamily(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');
        $first = $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');

        try {
            $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');
            self::fail('Rotating a token used within the last 10 seconds must throw refresh_invalid.');
        } catch (AuthException $error) {
            self::assertSame('refresh_invalid', $error->errorCode);
        }

        // Nothing was revoked, so the replacement from the first rotation still works.
        $second = $this->refresh->rotate($first['token'], 'web', '127.0.0.1');
        self::assertSame($issued['family_id'], $second['family_id']);
    }

    public function testRotatingAUsedTokenAfterTheGraceWindowIsReuseAndRevokesTheFamily(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');
        $newer = $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');
        $this->pdo()->prepare(
            'UPDATE refresh_tokens SET used_at = DATE_SUB(NOW(6), INTERVAL 11 SECOND) WHERE token_hash = ?'
        )->execute([Tokens::hash($issued['token'])]);

        try {
            $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');
            self::fail('Rotating a token used more than 10 seconds ago must throw refresh_reused.');
        } catch (AuthException $error) {
            self::assertSame('refresh_reused', $error->errorCode);
        }

        try {
            $this->refresh->rotate($newer['token'], 'web', '127.0.0.1');
            self::fail('The family must be revoked after reuse.');
        } catch (AuthException $error) {
            self::assertSame('refresh_invalid', $error->errorCode);
        }
    }

    public function testWrongClientValueIsInvalid(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');

        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('refresh_invalid');

        $this->refresh->rotate($issued['token'], 'launcher', '127.0.0.1');
    }

    public function testMalformedTokenIsInvalid(): void
    {
        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('refresh_invalid');

        $this->refresh->rotate('nope', 'web', '127.0.0.1');
    }

    public function testIdleExpiredTokenThrowsRefreshExpired(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');
        $this->pdo()->prepare(
            'UPDATE refresh_tokens SET idle_expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE user_id = ?'
        )->execute([$this->userId]);

        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('refresh_expired');

        $this->refresh->rotate($issued['token'], 'web', '127.0.0.1');
    }

    public function testRevokeAllExceptFamilyKeepsTheGivenFamilyOnly(): void
    {
        $kept = $this->refresh->issue($this->userId, 'web');
        $other = $this->refresh->issue($this->userId, 'web');

        $this->refresh->revokeAllExceptFamily($this->userId, $kept['family_id']);

        $rotated = $this->refresh->rotate($kept['token'], 'web', '127.0.0.1');
        self::assertSame($kept['family_id'], $rotated['family_id']);

        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('refresh_invalid');

        $this->refresh->rotate($other['token'], 'web', '127.0.0.1');
    }

    public function testRevokeAllExceptFamilyLeavesOtherUsersAlone(): void
    {
        $otherUser = (new UserRepository())->create('other@example.test', null, null);
        $foreign = $this->refresh->issue($otherUser, 'web');
        $mine = $this->refresh->issue($this->userId, 'web');

        $this->refresh->revokeAllExceptFamily($this->userId, $mine['family_id']);

        $rotated = $this->refresh->rotate($foreign['token'], 'web', '127.0.0.1');
        self::assertSame($otherUser, $rotated['user_id']);
    }

    public function testRevokeByTokenReturnsUserAndFamily(): void
    {
        $issued = $this->refresh->issue($this->userId, 'web');

        $revoked = $this->refresh->revokeByToken($issued['token']);

        self::assertSame(['user_id' => $this->userId, 'family_id' => $issued['family_id']], $revoked);
    }

    public function testRevokeByUnknownTokenReturnsNull(): void
    {
        self::assertNull($this->refresh->revokeByToken(Tokens::random()));
    }

    public function testRevokeByMalformedTokenReturnsNull(): void
    {
        self::assertNull($this->refresh->revokeByToken('garbage'));
    }

    public function testRevokeAllForUserEndsTheDeviceOfEachFamily(): void
    {
        $device = (new Devices())->add($this->userId, 'Drew PC', 'Windows 11', '1.4.0');
        $issued = $this->refresh->issue($this->userId, 'launcher', $device);

        $this->refresh->revokeAllForUser($this->userId);

        self::assertTrue($this->deviceRevoked($device));

        try {
            $this->refresh->rotate($issued['token'], 'launcher', '127.0.0.1');
            self::fail('The family must be revoked with its device.');
        } catch (AuthException $error) {
            self::assertSame('refresh_invalid', $error->errorCode);
        }
    }

    public function testRevokeFamilyForUserEndsOnlyThatFamiliesDevice(): void
    {
        $ended = (new Devices())->add($this->userId, 'Drew PC', 'Windows 11', '1.4.0');
        $kept = (new Devices())->add($this->userId, 'Laptop', 'Linux', '1.4.0');
        $endedFamily = $this->refresh->issue($this->userId, 'launcher', $ended);
        $keptFamily = $this->refresh->issue($this->userId, 'launcher', $kept);

        self::assertTrue($this->refresh->revokeFamilyForUser($this->userId, $endedFamily['family_id']));

        self::assertTrue($this->deviceRevoked($ended));
        self::assertFalse($this->deviceRevoked($kept));
        self::assertSame(
            $this->userId,
            $this->refresh->rotate($keptFamily['token'], 'launcher', '127.0.0.1')['user_id'],
        );
    }

    public function testRevokeAllExceptFamilyRunsInsideTheCallersTransaction(): void
    {
        $kept = $this->refresh->issue($this->userId, 'web');
        $other = $this->refresh->issue($this->userId, 'web');

        Database::transaction(function () use ($kept): void {
            $this->refresh->revokeAllExceptFamily($this->userId, $kept['family_id']);
        });

        $rotated = $this->refresh->rotate($kept['token'], 'web', '127.0.0.1');
        self::assertSame($kept['family_id'], $rotated['family_id']);

        try {
            $this->refresh->rotate($other['token'], 'web', '127.0.0.1');
            self::fail('The other family must be revoked.');
        } catch (AuthException $error) {
            self::assertSame('refresh_invalid', $error->errorCode);
        }
    }

    public function testRevocationInsideACallersTransactionRollsBackWithIt(): void
    {
        $device = (new Devices())->add($this->userId, 'Drew PC', 'Windows 11', '1.4.0');
        $issued = $this->refresh->issue($this->userId, 'launcher', $device);

        try {
            Database::transaction(function (): void {
                $this->refresh->revokeAllForUser($this->userId);

                throw new \RuntimeException('The caller fails after the revocation.');
            });
            self::fail('The caller exception must reach the test.');
        } catch (\RuntimeException) {
            // Expected: the caller's transaction is rolled back.
        }

        self::assertFalse($this->deviceRevoked($device));
        $rotated = $this->refresh->rotate($issued['token'], 'launcher', '127.0.0.1');
        self::assertSame($issued['family_id'], $rotated['family_id']);
    }

    private function deviceRevoked(int $deviceId): bool
    {
        $statement = $this->pdo()->prepare('SELECT revoked_at FROM devices WHERE id = ?');
        $statement->execute([$deviceId]);

        // A missing row fetches false and an active device fetches null, so only a timestamp counts as revoked.
        return is_string($statement->fetchColumn());
    }
}
