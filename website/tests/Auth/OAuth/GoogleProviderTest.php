<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\OAuth\GoogleProvider;
use app\Auth\OAuth\ProviderIdentity;
use app\Config\Config;
use PHPUnit\Framework\TestCase;

final class GoogleProviderTest extends TestCase
{
    private const CLIENT_ID = 'test-google-client-id';
    private const ISSUER = 'https://accounts.google.com';
    private const JWKS_URL = 'https://www.googleapis.com/oauth2/v3/certs';
    private const TOKEN_URL = 'https://oauth2.googleapis.com/token';
    private const REDIRECT_URI = 'https://app.test/api/auth/google/callback';
    private const NONCE = 'started-nonce-value';

    private SigningKey $key;

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('GOOGLE_CLIENT_ID', self::CLIENT_ID);
        Config::set('GOOGLE_CLIENT_SECRET', 'test-google-client-secret');

        $this->key = SigningKey::generate('google-provider-test-key');
    }

    protected function tearDown(): void
    {
        Config::reset();

        parent::tearDown();
    }

    public function testTheAuthorizeUrlCarriesTheStateNonceChallengeAndSelectAccountPrompt(): void
    {
        $url = (new GoogleProvider())->authorizeUrl('the-state', 'the-challenge', self::NONCE, self::REDIRECT_URI);

        self::assertStringStartsWith('https://accounts.google.com/o/oauth2/v2/auth?', $url);
        parse_str((string) parse_url($url, PHP_URL_QUERY), $query);
        self::assertSame(self::CLIENT_ID, $query['client_id']);
        self::assertSame('the-state', $query['state']);
        self::assertSame(self::NONCE, $query['nonce']);
        self::assertSame('the-challenge', $query['code_challenge']);
        self::assertSame('S256', $query['code_challenge_method']);
        self::assertSame('select_account', $query['prompt']);
    }

    public function testABooleanTrueEmailVerifiedGivesAVerifiedIdentity(): void
    {
        self::assertTrue($this->identity(['email_verified' => true])->emailVerified);
    }

    public function testAStringTrueEmailVerifiedGivesAVerifiedIdentity(): void
    {
        self::assertTrue($this->identity(['email_verified' => 'true'])->emailVerified);
    }

    public function testAStringFalseEmailVerifiedDoesNotCountAsVerified(): void
    {
        self::assertFalse($this->identity(['email_verified' => 'false'])->emailVerified);
    }

    public function testABooleanFalseEmailVerifiedGivesAnUnverifiedIdentity(): void
    {
        self::assertFalse($this->identity(['email_verified' => false])->emailVerified);
    }

    public function testAMissingEmailVerifiedClaimGivesAnUnverifiedIdentity(): void
    {
        self::assertFalse($this->identity([], ['email_verified'])->emailVerified);
    }

    public function testAWrongNonceIsRefused(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token nonce does not match');

        (new GoogleProvider())->identity(
            ['id_token' => $this->idToken()],
            'a-different-nonce',
            $this->transport(),
        );
    }

    public function testATokenResponseWithoutAnIdTokenIsRefused(): void
    {
        $transport = new FakeTransport([self::TOKEN_URL => FakeTransport::json(['access_token' => 'provider-access'])]);

        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('Google token response has no ID token');

        (new GoogleProvider())->exchange('provider-code', 'the-verifier', self::REDIRECT_URI, $transport);
    }

    private function identity(array $overrides = [], array $omit = []): ProviderIdentity
    {
        return (new GoogleProvider())->identity(
            ['id_token' => $this->idToken($overrides, $omit)],
            self::NONCE,
            $this->transport(),
        );
    }

    private function transport(): FakeTransport
    {
        return new FakeTransport([self::JWKS_URL => FakeTransport::json($this->key->jwks())]);
    }

    /**
     * @param array<string, mixed> $overrides
     * @param list<string> $omit Claim names to leave out of the token.
     */
    private function idToken(array $overrides = [], array $omit = []): string
    {
        $now = time();
        $claims = array_merge([
            'iss' => self::ISSUER,
            'aud' => self::CLIENT_ID,
            'sub' => 'google-subject-1',
            'email' => 'pilot@example.test',
            'email_verified' => true,
            'name' => 'Pilot',
            'iat' => $now,
            'exp' => $now + 300,
            'nonce' => self::NONCE,
        ], $overrides);

        return $this->key->sign(array_diff_key($claims, array_flip($omit)));
    }
}
