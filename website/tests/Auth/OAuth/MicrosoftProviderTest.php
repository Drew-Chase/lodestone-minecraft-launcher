<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\OAuth\MicrosoftProvider;
use app\Config\Config;
use PHPUnit\Framework\TestCase;

final class MicrosoftProviderTest extends TestCase
{
    private const CLIENT_ID = 'test-microsoft-client-id';
    private const ISSUER = 'https://login.microsoftonline.com/9188040d-6c67-4c5b-b112-36a304b66dad/v2.0';
    private const JWKS_URL = 'https://login.microsoftonline.com/consumers/discovery/v2.0/keys';
    private const REDIRECT_URI = 'https://app.test/api/auth/microsoft/callback';
    private const NONCE = 'started-nonce-value';

    private SigningKey $key;
    private FakeTransport $transport;

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('MICROSOFT_CLIENT_ID', self::CLIENT_ID);
        Config::set('MICROSOFT_CLIENT_SECRET', 'test-microsoft-client-secret');

        $this->key = SigningKey::generate('microsoft-test-key');
        $this->transport = new FakeTransport([self::JWKS_URL => FakeTransport::json($this->key->jwks())]);
    }

    protected function tearDown(): void
    {
        Config::reset();

        parent::tearDown();
    }

    public function testTheAuthorizeUrlUsesTheConsumersTenantWithStateNonceAndAS256Challenge(): void
    {
        $url = (new MicrosoftProvider())->authorizeUrl('the-state', 'the-challenge', self::NONCE, self::REDIRECT_URI);

        self::assertStringStartsWith('https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?', $url);
        parse_str((string) parse_url($url, PHP_URL_QUERY), $query);
        self::assertSame(self::CLIENT_ID, $query['client_id']);
        self::assertSame('code', $query['response_type']);
        self::assertSame(self::REDIRECT_URI, $query['redirect_uri']);
        self::assertSame('the-state', $query['state']);
        self::assertSame(self::NONCE, $query['nonce']);
        self::assertSame('the-challenge', $query['code_challenge']);
        self::assertSame('S256', $query['code_challenge_method']);
    }

    public function testAnIdTokenWithTheRightIssuerAudienceAndNonceGivesTheIdentity(): void
    {
        $identity = (new MicrosoftProvider())->identity(
            ['id_token' => $this->idToken()],
            self::NONCE,
            $this->transport,
        );

        self::assertSame('microsoft', $identity->provider);
        self::assertSame('ms-subject-1', $identity->subject);
        self::assertSame('pilot@outlook.test', $identity->email);
        self::assertTrue($identity->emailVerified);
        self::assertSame('Pilot', $identity->name);
    }

    public function testAnIdTokenWithoutANameGivesAnIdentityWithANullName(): void
    {
        $identity = (new MicrosoftProvider())->identity(
            ['id_token' => $this->idToken(['name' => null])],
            self::NONCE,
            $this->transport,
        );

        self::assertNull($identity->name);
    }

    public function testTheKeySetIsReadFromTheConsumersEndpoint(): void
    {
        (new MicrosoftProvider())->identity(['id_token' => $this->idToken()], self::NONCE, $this->transport);

        $requests = $this->transport->requests();
        self::assertCount(1, $requests);
        self::assertSame('GET', $requests[0]['method']);
        self::assertSame(self::JWKS_URL, $requests[0]['url']);
    }

    public function testAWrongNonceIsRefused(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token nonce does not match');

        (new MicrosoftProvider())->identity(['id_token' => $this->idToken()], 'a-different-nonce', $this->transport);
    }

    public function testAWrongIssuerIsRefused(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token issuer is not accepted');

        (new MicrosoftProvider())->identity(
            ['id_token' => $this->idToken(['iss' => 'https://login.microsoftonline.com/common/v2.0'])],
            self::NONCE,
            $this->transport,
        );
    }

    public function testAMissingIdTokenIsRefused(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('Microsoft sign-in has no ID token');

        (new MicrosoftProvider())->identity(['id_token' => ''], self::NONCE, $this->transport);
    }

    /** @param array<string, mixed> $overrides */
    private function idToken(array $overrides = []): string
    {
        $now = time();

        return $this->key->sign(array_merge([
            'iss' => self::ISSUER,
            'aud' => self::CLIENT_ID,
            'sub' => 'ms-subject-1',
            'email' => 'pilot@outlook.test',
            'name' => 'Pilot',
            'iat' => $now,
            'exp' => $now + 300,
            'nonce' => self::NONCE,
        ], $overrides));
    }
}
