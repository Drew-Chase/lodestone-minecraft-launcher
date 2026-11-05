<?php

declare(strict_types=1);

namespace tests\Mail;

use app\Mail\AccountMail;
use PHPUnit\Framework\TestCase;

final class AccountMailTest extends TestCase
{
    private const TOKEN = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';

    public function testVerificationLinkCarriesTheTokenInTheFragment(): void
    {
        $mail = AccountMail::verification('http://localhost:3000', self::TOKEN);

        self::assertStringContainsString(
            'http://localhost:3000/verify-email#token=' . self::TOKEN,
            $mail['text']
        );
        self::assertStringNotContainsString('verify-email?token=', $mail['text']);
    }

    public function testVerificationHtmlContainsTheSameLink(): void
    {
        $mail = AccountMail::verification('http://localhost:3000', self::TOKEN);

        self::assertStringContainsString(
            'http://localhost:3000/verify-email#token=' . self::TOKEN,
            $mail['html']
        );
    }

    public function testTrailingSlashOnTheAppUrlIsNotDoubled(): void
    {
        $mail = AccountMail::verification('http://localhost:3000/', self::TOKEN);

        self::assertStringContainsString('http://localhost:3000/verify-email#token=', $mail['text']);
        self::assertStringNotContainsString('//verify-email', $mail['text']);
    }

    public function testPasswordResetLinkUsesTheResetPathAndFragment(): void
    {
        $mail = AccountMail::passwordReset('http://localhost:3000', self::TOKEN);

        self::assertStringContainsString('http://localhost:3000/reset-password#token=' . self::TOKEN, $mail['text']);
        self::assertStringContainsString('http://localhost:3000/reset-password#token=' . self::TOKEN, $mail['html']);
        self::assertStringNotContainsString('reset-password?token=', $mail['text']);
    }

    public function testFinishRegistrationHasItsSubject(): void
    {
        $mail = AccountMail::finishRegistration('http://localhost:3000', self::TOKEN);

        self::assertSame('Finish creating your Lodestone account', $mail['subject']);
    }

    public function testFinishRegistrationLinkIsAResetLinkWithTokenInTheFragment(): void
    {
        $mail = AccountMail::finishRegistration('http://localhost:3000', self::TOKEN);

        self::assertStringContainsString('http://localhost:3000/reset-password#token=' . self::TOKEN, $mail['text']);
        self::assertStringContainsString('http://localhost:3000/reset-password#token=' . self::TOKEN, $mail['html']);
    }

    public function testFinishRegistrationTextSaysWhatHappenedAndHowLongTheLinkLasts(): void
    {
        $mail = AccountMail::finishRegistration('http://localhost:3000', self::TOKEN);

        self::assertStringContainsString('Someone asked to create a Lodestone account with this email address', $mail['text']);
        self::assertStringContainsString('An account for it already exists', $mail['text']);
        self::assertStringContainsString('valid for 24 hours', $mail['text']);
        self::assertStringContainsString('choose a password', $mail['text']);
        self::assertStringContainsString('you can ignore this email', $mail['text']);
    }

    public function testFinishRegistrationHtmlCarriesTheSameMessage(): void
    {
        $mail = AccountMail::finishRegistration('http://localhost:3000', self::TOKEN);

        self::assertStringContainsString('An account for it already exists', $mail['html']);
        self::assertStringContainsString('valid for 24 hours', $mail['html']);
        self::assertStringContainsString('you can ignore this email', $mail['html']);
    }

    public function testFinishRegistrationEscapesTheAppUrlInHtml(): void
    {
        $mail = AccountMail::finishRegistration('http://x.test/"><script>alert(1)</script>', self::TOKEN);

        self::assertStringNotContainsString('<script>', $mail['html']);
        self::assertStringContainsString('&lt;script&gt;', $mail['html']);
    }

    public function testHostileAppUrlIsEscapedInHtml(): void
    {
        $hostile = 'http://x.test/"><script>alert(1)</script>';

        $mail = AccountMail::verification($hostile, self::TOKEN);

        self::assertStringNotContainsString('<script>', $mail['html']);
        self::assertStringContainsString('&lt;script&gt;', $mail['html']);
    }

    public function testAlreadyRegisteredHasNoToken(): void
    {
        $mail = AccountMail::alreadyRegistered('http://localhost:3000');

        self::assertStringNotContainsString('token=', $mail['text']);
        self::assertStringNotContainsString('token=', $mail['html']);
    }

    public function testAlreadyRegisteredPointsAtSignInAndReset(): void
    {
        $mail = AccountMail::alreadyRegistered('http://localhost:3000');

        self::assertStringContainsString('http://localhost:3000/login', $mail['text']);
        self::assertStringContainsString('http://localhost:3000/forgot-password', $mail['text']);
        self::assertSame('Your Lodestone account already exists', $mail['subject']);
    }

    public function testAlreadyRegisteredEscapesTheAppUrlInHtml(): void
    {
        $mail = AccountMail::alreadyRegistered('http://x.test/<b>');

        self::assertStringNotContainsString('<b>', $mail['html']);
    }
}
