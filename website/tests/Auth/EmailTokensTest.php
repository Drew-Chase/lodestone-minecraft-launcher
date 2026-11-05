<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\AuthException;
use app\Auth\EmailTokens;
use app\Users\UserRepository;
use tests\Support\DatabaseTestCase;

final class EmailTokensTest extends DatabaseTestCase
{
    private EmailTokens $tokens;
    private int $userId;

    protected function setUp(): void
    {
        parent::setUp();

        $this->tokens = new EmailTokens();
        $this->userId = (new UserRepository())->create('tokens@example.test', null, null);
    }

    public function testCreateThenConsumeReturnsTheUserId(): void
    {
        $raw = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);

        self::assertSame($this->userId, $this->tokens->consume($raw, EmailTokens::VERIFY));
    }

    public function testCreatedTokenIsSixtyFourHexCharacters(): void
    {
        $raw = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);

        self::assertMatchesRegularExpression('/^[a-f0-9]{64}$/', $raw);
    }

    public function testSecondConsumeOfTheSameTokenIsInvalid(): void
    {
        $raw = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);
        $this->tokens->consume($raw, EmailTokens::VERIFY);

        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('invalid_token');

        $this->tokens->consume($raw, EmailTokens::VERIFY);
    }

    public function testTokenOfOnePurposeCannotBeUsedForAnother(): void
    {
        $raw = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);

        try {
            $this->tokens->consume($raw, EmailTokens::RESET);
            self::fail('A verify token must not be accepted as a reset token.');
        } catch (AuthException $error) {
            self::assertSame('invalid_token', $error->errorCode);
        }

        // The wrong purpose does not spend the token, so the right purpose still works.
        self::assertSame($this->userId, $this->tokens->consume($raw, EmailTokens::VERIFY));
    }

    public function testMalformedTokenIsInvalid(): void
    {
        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('invalid_token');

        $this->tokens->consume('not-a-token', EmailTokens::VERIFY);
    }

    public function testExpiredTokenThrowsTokenExpired(): void
    {
        $raw = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);
        $this->pdo()->prepare(
            'UPDATE email_tokens SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE user_id = ?'
        )->execute([$this->userId]);

        $this->expectException(AuthException::class);
        $this->expectExceptionMessage('token_expired');

        $this->tokens->consume($raw, EmailTokens::VERIFY);
    }

    public function testCreatingASecondTokenInvalidatesTheFirst(): void
    {
        $first = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);
        $second = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);

        try {
            $this->tokens->consume($first, EmailTokens::VERIFY);
            self::fail('The first token must be invalid once a second one exists.');
        } catch (AuthException $error) {
            self::assertSame('invalid_token', $error->errorCode);
        }

        self::assertSame($this->userId, $this->tokens->consume($second, EmailTokens::VERIFY));
    }

    public function testNewTokenOfOnePurposeLeavesTheOtherPurposeUntouched(): void
    {
        $verify = $this->tokens->create($this->userId, EmailTokens::VERIFY, 3600);
        $this->tokens->create($this->userId, EmailTokens::RESET, 3600);

        self::assertSame($this->userId, $this->tokens->consume($verify, EmailTokens::VERIFY));
    }
}
