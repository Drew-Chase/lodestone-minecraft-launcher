<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\AuditLog;
use app\Auth\AuthException;
use app\Auth\AuthService;
use app\Auth\EmailTokens;
use app\Auth\JwtIssuer;
use app\Auth\NullBreachChecker;
use app\Auth\PasswordHasher;
use app\Auth\PasswordPolicy;
use app\Auth\RateLimiter;
use app\Auth\RefreshTokens;
use app\Auth\Tokens;
use app\Mail\Mailer;
use app\Mail\MemoryMailer;
use app\Users\UserRepository;
use tests\Support\DatabaseTestCase;

final class AuthServiceTest extends DatabaseTestCase
{
    private const EMAIL = 'pilot@example.test';
    private const PASSWORD = 'correct horse battery';
    private const NEW_PASSWORD = 'another long passphrase';
    private const IP = '198.51.100.7';
    private const SECRET = 'AUTH_SERVICE_TEST_SECRET_0123456789';

    private MemoryMailer $mailer;
    private UserRepository $users;
    private AuthService $auth;

    protected function setUp(): void
    {
        parent::setUp();

        $this->mailer = new MemoryMailer();
        $this->mailer->reset();
        $this->users = new UserRepository();
        $this->auth = $this->service($this->mailer);
    }

    public function testVerifyEmailWithEmailedTokenMarksTheUserVerified(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $token = $this->tokenFromLastMail();

        $this->auth->verifyEmail($token, self::IP);

        self::assertNotNull($this->users->findByEmail(self::EMAIL)['email_verified_at']);
    }

    public function testVerificationLinkUsesTheAppUrl(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);

        $sent = $this->mailer->sent();
        self::assertCount(1, $sent);
        self::assertSame(self::EMAIL, $sent[0]['to']);
        self::assertStringContainsString('http://localhost:3000/verify-email#token=', $sent[0]['text']);
        self::assertStringNotContainsString('verify-email?token=', $sent[0]['text']);
    }

    public function testVerifyingTheSameTokenTwiceIsInvalid(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $token = $this->tokenFromLastMail();
        $this->auth->verifyEmail($token, self::IP);

        $this->assertCode('invalid_token', fn () => $this->auth->verifyEmail($token, self::IP));
    }

    public function testRegisterForVerifiedAddressSendsAlreadyRegisteredWithoutAToken(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $this->auth->verifyEmail($this->tokenFromLastMail(), self::IP);
        $this->mailer->reset();

        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);

        $sent = $this->mailer->sent();
        self::assertCount(1, $sent);
        self::assertSame('Your Lodestone account already exists', $sent[0]['subject']);
        self::assertStringNotContainsString('token=', $sent[0]['text']);
        self::assertSame(1, $this->tokenCount($this->users->findByEmail(self::EMAIL)['id']));
    }

    public function testRegisterForUnverifiedAddressSendsFinishRegistrationAndKeepsThePassword(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $this->mailer->reset();

        $this->auth->register(self::EMAIL, self::NEW_PASSWORD, null, self::IP);

        $sent = $this->mailer->sent();
        self::assertCount(1, $sent);
        self::assertSame(self::EMAIL, $sent[0]['to']);
        self::assertSame('Finish creating your Lodestone account', $sent[0]['subject']);
        self::assertStringContainsString('/reset-password#token=', $sent[0]['text']);

        // The second registration named a different password, and it must not have replaced the first.
        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);
        self::assertNotEmpty($session['access_token']);
        $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, self::NEW_PASSWORD, self::IP));
    }

    public function testFinishRegistrationLinkSetsThePasswordAndVerifiesTheAddress(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $this->auth->register(self::EMAIL, self::NEW_PASSWORD, null, self::IP);

        $this->auth->resetPassword($this->tokenFromLastMail(), self::NEW_PASSWORD, self::IP);

        self::assertNotNull($this->users->findByEmail(self::EMAIL)['email_verified_at']);
        self::assertNotNull($this->auth->login(self::EMAIL, self::NEW_PASSWORD, self::IP)['access_token']);
        $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, self::PASSWORD, self::IP));
    }

    public function testResendVerificationForUnverifiedAccountSendsFinishRegistration(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $this->mailer->reset();

        $this->auth->resendVerification(self::EMAIL, self::IP);

        $sent = $this->mailer->sent();
        self::assertCount(1, $sent);
        self::assertSame(self::EMAIL, $sent[0]['to']);
        self::assertSame('Finish creating your Lodestone account', $sent[0]['subject']);
        self::assertStringContainsString('/reset-password#token=', $sent[0]['text']);
    }

    public function testVerifyEmailRevokesExistingSessions(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $verifyToken = $this->tokenFromLastMail();
        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $this->auth->verifyEmail($verifyToken, self::IP);

        $error = $this->catchAuth(fn () => $this->auth->refresh($session['refresh_token'], 'web', self::IP));
        self::assertContains($error->errorCode, ['refresh_invalid', 'refresh_reused']);
    }

    public function testTenRegistersFromOneAddressAreAllowed(): void
    {
        for ($i = 1; $i <= 10; $i++) {
            $this->auth->register("user{$i}@example.test", self::PASSWORD, null, self::IP);
        }

        self::assertNotNull($this->users->findByEmail('user10@example.test'));
    }

    public function testEleventhRegisterFromOneAddressIsRateLimited(): void
    {
        for ($i = 1; $i <= 10; $i++) {
            $this->auth->register("user{$i}@example.test", self::PASSWORD, null, self::IP);
        }

        $error = $this->catchAuth(fn () => $this->auth->register('user11@example.test', self::PASSWORD, null, self::IP));

        self::assertSame('rate_limited', $error->errorCode);
        self::assertSame(429, $error->httpStatus);
        self::assertNotNull($error->retryAfter);
        self::assertGreaterThanOrEqual(1, $error->retryAfter);
        self::assertLessThanOrEqual(3600, $error->retryAfter);
    }

    public function testLoginReturnsSessionAndAccessToken(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);

        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        self::assertSame(900, $session['expires_in']);
        self::assertSame(self::EMAIL, $session['user']['email']);
        self::assertNotEmpty($session['refresh_token']);
        self::assertMatchesRegularExpression('/^[a-f0-9]{32}$/', $session['family_id']);

        $claims = $this->jwt()->verify($session['access_token']);
        self::assertSame($session['user']['id'], $claims['user_id']);
        self::assertSame($session['family_id'], $claims['family_id']);
    }

    public function testLoginWorksBeforeVerification(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);

        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        self::assertFalse($session['user']['email_verified']);
    }

    public function testLoginWithWrongPasswordIsInvalidCredentials(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);

        $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, 'wrong password here', self::IP));
    }

    public function testLoginForMissingAddressIsInvalidCredentials(): void
    {
        $this->assertCode('invalid_credentials', fn () => $this->auth->login('nobody@example.test', self::PASSWORD, self::IP));
    }

    public function testSixthLoginAfterFiveFailuresIsRateLimited(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        for ($i = 0; $i < 5; $i++) {
            $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, 'wrong password here', self::IP));
        }

        $error = $this->catchAuth(fn () => $this->auth->login(self::EMAIL, self::PASSWORD, self::IP));

        self::assertSame('rate_limited', $error->errorCode);
        self::assertGreaterThanOrEqual(1, $error->retryAfter);
    }

    public function testSuccessfulLoginClearsTheAddressFailureCount(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);

        // Each attempt comes from its own client address, so the per-address limit is the only one in play.
        for ($i = 0; $i < 4; $i++) {
            $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, 'wrong password here', "198.51.100.{$i}0"));
        }
        $this->auth->login(self::EMAIL, self::PASSWORD, '198.51.100.50');

        for ($i = 0; $i < 4; $i++) {
            $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, 'wrong password here', "198.51.101.{$i}0"));
        }
    }

    public function testRefreshReturnsANewRefreshToken(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $refreshed = $this->auth->refresh($session['refresh_token'], 'web', self::IP);

        self::assertNotSame($session['refresh_token'], $refreshed['refresh_token']);
        self::assertSame($session['family_id'], $refreshed['family_id']);
        self::assertSame(900, $refreshed['expires_in']);
    }

    public function testRefreshWithOldTokenIsInvalidWithinGraceAndReuseAfterIt(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);
        $this->auth->refresh($session['refresh_token'], 'web', self::IP);

        // Within the grace window a repeat is refused without revoking anything.
        $this->assertCode('refresh_invalid', fn () => $this->auth->refresh($session['refresh_token'], 'web', self::IP));

        // Past the window the same token is reuse. Backdate its rotation time to simulate that.
        $backdate = $this->pdo()->prepare(
            'UPDATE refresh_tokens SET used_at = DATE_SUB(NOW(6), INTERVAL 11 SECOND) WHERE token_hash = ?'
        );
        $backdate->execute([Tokens::hash($session['refresh_token'])]);

        $this->assertCode('refresh_reused', fn () => $this->auth->refresh($session['refresh_token'], 'web', self::IP));
    }

    public function testLogoutRevokesTheFamily(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $session = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $this->auth->logout($session['refresh_token'], self::IP);

        $this->assertCode('refresh_invalid', fn () => $this->auth->refresh($session['refresh_token'], 'web', self::IP));
    }

    public function testLogoutWithGarbageTokenDoesNotThrowAndSendsNoMail(): void
    {
        $this->registerAndVerify();
        $this->mailer->reset();

        $this->auth->logout('garbage', self::IP);
        $this->auth->logout(str_repeat('a', 64), self::IP);

        self::assertCount(0, $this->mailer->sent());
    }

    public function testPasswordResetForUnknownAddressSendsNothing(): void
    {
        $this->auth->requestPasswordReset('nobody@example.test', self::IP);

        self::assertSame([], $this->mailer->sent());
    }

    public function testPasswordResetRoundTripChangesThePassword(): void
    {
        $this->registerAndVerify();
        $this->mailer->reset();

        $this->auth->requestPasswordReset(self::EMAIL, self::IP);
        $sent = $this->mailer->sent();
        self::assertCount(1, $sent);
        self::assertSame('Reset your Lodestone password', $sent[0]['subject']);

        $this->auth->resetPassword($this->tokenFromLastMail(), self::NEW_PASSWORD, self::IP);

        self::assertNotNull($this->auth->login(self::EMAIL, self::NEW_PASSWORD, self::IP)['access_token']);
        $this->assertCode('invalid_credentials', fn () => $this->auth->login(self::EMAIL, self::PASSWORD, self::IP));
    }

    public function testPasswordResetAlsoMarksTheEmailVerified(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $this->mailer->reset();
        $this->auth->requestPasswordReset(self::EMAIL, self::IP);

        $this->auth->resetPassword($this->tokenFromLastMail(), self::NEW_PASSWORD, self::IP);

        self::assertNotNull($this->users->findByEmail(self::EMAIL)['email_verified_at']);
    }

    public function testWeakPasswordDoesNotSpendTheResetToken(): void
    {
        $this->registerAndVerify();
        $this->mailer->reset();
        $this->auth->requestPasswordReset(self::EMAIL, self::IP);
        $token = $this->tokenFromLastMail();

        $this->assertCode('password_too_short', fn () => $this->auth->resetPassword($token, 'short', self::IP));

        $this->auth->resetPassword($token, self::NEW_PASSWORD, self::IP);
        self::assertNotNull($this->auth->login(self::EMAIL, self::NEW_PASSWORD, self::IP)['access_token']);
    }

    public function testChangePasswordWithWrongCurrentPasswordIsInvalidCredentials(): void
    {
        $session = $this->registerAndLogin();

        $this->assertCode('invalid_credentials', fn () => $this->auth->changePassword(
            $session['user']['id'], 'not the password', self::NEW_PASSWORD, $session['family_id'], self::IP,
        ));
    }

    public function testChangePasswordRevokesOtherSessionsAndKeepsTheCurrentOne(): void
    {
        $current = $this->registerAndLogin();
        $other = $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);

        $this->auth->changePassword($current['user']['id'], self::PASSWORD, self::NEW_PASSWORD, $current['family_id'], self::IP);

        $this->assertCode('refresh_invalid', fn () => $this->auth->refresh($other['refresh_token'], 'web', self::IP));
        $kept = $this->auth->refresh($current['refresh_token'], 'web', self::IP);
        self::assertSame($current['family_id'], $kept['family_id']);
        self::assertNotNull($this->auth->login(self::EMAIL, self::NEW_PASSWORD, self::IP)['access_token']);
    }

    public function testCurrentUserForMissingUserIsUnauthorized(): void
    {
        $this->assertCode('unauthorized', fn () => $this->auth->currentUser(999999));
    }

    public function testCurrentUserReturnsThePublicShape(): void
    {
        $session = $this->registerAndLogin();

        $user = $this->auth->currentUser($session['user']['id']);

        self::assertSame(self::EMAIL, $user['email']);
        self::assertArrayNotHasKey('password_hash', $user);
    }

    public function testMailerFailureDoesNotMakeRegisterThrow(): void
    {
        $throwing = $this->service(new class implements Mailer {
            public function send(string $to, string $subject, string $text, string $html): void
            {
                throw new \RuntimeException('smtp unavailable');
            }
        });

        $throwing->register(self::EMAIL, self::PASSWORD, null, self::IP);

        self::assertNotNull($this->users->findByEmail(self::EMAIL));
    }

    private function service(Mailer $mailer): AuthService
    {
        return new AuthService(
            new UserRepository(),
            new RefreshTokens(),
            new EmailTokens(),
            new RateLimiter(),
            new AuditLog(),
            new PasswordHasher(),
            new PasswordPolicy(new NullBreachChecker()),
            $this->jwt(),
            $mailer,
            'http://localhost:3000',
        );
    }

    private function jwt(): JwtIssuer
    {
        return new JwtIssuer(self::SECRET, 'https://issuer.test');
    }

    private function registerAndVerify(): void
    {
        $this->auth->register(self::EMAIL, self::PASSWORD, null, self::IP);
        $this->auth->verifyEmail($this->tokenFromLastMail(), self::IP);
    }

    /** @return array{user: array<string, mixed>, access_token: string, expires_in: int, refresh_token: string, family_id: string} */
    private function registerAndLogin(): array
    {
        $this->registerAndVerify();

        return $this->auth->login(self::EMAIL, self::PASSWORD, self::IP);
    }

    private function tokenFromLastMail(): string
    {
        $sent = $this->mailer->sent();
        self::assertNotEmpty($sent, 'Expected an account mail.');

        preg_match('/token=([a-f0-9]{64})/', $sent[array_key_last($sent)]['text'], $matches);
        self::assertArrayHasKey(1, $matches, 'The mail carries no token.');

        return $matches[1];
    }

    private function tokenCount(int|string $userId): int
    {
        $statement = $this->pdo()->prepare('SELECT COUNT(*) FROM email_tokens WHERE user_id = ?');
        $statement->execute([$userId]);

        return (int) $statement->fetchColumn();
    }

    private function catchAuth(callable $call): AuthException
    {
        try {
            $call();
        } catch (AuthException $error) {
            return $error;
        }

        self::fail('Expected an AuthException.');
    }

    private function assertCode(string $code, callable $call): void
    {
        self::assertSame($code, $this->catchAuth($call)->errorCode);
    }
}
