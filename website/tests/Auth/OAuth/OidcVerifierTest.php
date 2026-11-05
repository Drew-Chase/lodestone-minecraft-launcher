<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\OAuth\OidcVerifier;
use app\Auth\OAuth\UpstreamFailure;
use Firebase\JWT\JWT;
use PHPUnit\Framework\TestCase;

final class OidcVerifierTest extends TestCase
{
    private const ISSUER = 'https://issuer.test';
    private const AUDIENCE = 'test-client-id';
    private const NONCE = 'expected-nonce-value';

    private SigningKey $key;

    protected function setUp(): void
    {
        parent::setUp();

        $this->key = SigningKey::generate('verifier-test-key');
    }

    public function testValidTokenPassesAndReturnsItsClaims(): void
    {
        $claims = $this->verifier()->verify($this->token(), self::NONCE);

        self::assertSame('subject-1', $claims['sub']);
        self::assertSame(self::ISSUER, $claims['iss']);
    }

    public function testWrongAudienceFails(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token audience is not this client');

        $this->verifier()->verify($this->token(['aud' => 'another-client']), self::NONCE);
    }

    public function testWrongIssuerFails(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token issuer is not accepted');

        $this->verifier()->verify($this->token(['iss' => 'https://evil.test']), self::NONCE);
    }

    public function testWrongNonceFails(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token nonce does not match');

        $this->verifier()->verify($this->token(), 'a-different-nonce');
    }

    public function testExpiredTokenFails(): void
    {
        $this->expectException(\RuntimeException::class);
        // The JWT library rejects an expired token while it decodes, so the generic timing message is what callers see.
        $this->expectExceptionMessage('ID token signature or timing check failed');

        $this->verifier()->verify($this->token(['exp' => time() - 60]), self::NONCE);
    }

    public function testTokenSignedWithAnotherKeyFails(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token signature or timing check failed');

        // Same key ID as the published key, but a different key pair, so the signature cannot verify.
        $forged = SigningKey::generate('verifier-test-key')->sign($this->claims());

        $this->verifier()->verify($forged, self::NONCE);
    }

    public function testAnHs256TokenSignedWithThePublicKeyIsRefused(): void
    {
        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('ID token signature or timing check failed');

        // The published public key, used as an HMAC secret, is the classic algorithm confusion attack.
        $forged = JWT::encode($this->claims(), $this->key->publicPem(), 'HS256', 'verifier-test-key');

        $this->verifier()->verify($forged, self::NONCE);
    }

    public function testAKeySetThatCannotBeReadIsAnUpstreamFailure(): void
    {
        $this->expectException(UpstreamFailure::class);
        $this->expectExceptionMessage('Signing keys could not be read');

        $verifier = new OidcVerifier(
            static fn (): array => throw new \RuntimeException('down'),
            [self::ISSUER],
            self::AUDIENCE,
        );

        $verifier->verify($this->token(), self::NONCE);
    }

    public function testAKeySetWithNoKeysIsAnUpstreamFailure(): void
    {
        $this->expectException(UpstreamFailure::class);
        $this->expectExceptionMessage('No RS256 signing key is published');

        $verifier = new OidcVerifier(
            static fn (): array => ['keys' => []],
            [self::ISSUER],
            self::AUDIENCE,
        );

        $verifier->verify($this->token(), self::NONCE);
    }

    public function testAKeySetWithOnlyAnHs256KeyHasNoRs256KeyToUse(): void
    {
        $this->expectException(UpstreamFailure::class);
        $this->expectExceptionMessage('No RS256 signing key is published');

        // A static closure cannot read $this, so the key set is built first and captured.
        $jwks = ['keys' => [$this->hmacKey()]];
        $verifier = new OidcVerifier(static fn (): array => $jwks, [self::ISSUER], self::AUDIENCE);

        $verifier->verify($this->token(), self::NONCE);
    }

    public function testAnHs256KeyBesideTheRs256KeyStillLetsAValidTokenVerify(): void
    {
        $jwks = ['keys' => [$this->hmacKey(), $this->key->jwks()['keys'][0]]];
        $verifier = new OidcVerifier(static fn (): array => $jwks, [self::ISSUER], self::AUDIENCE);

        $claims = $verifier->verify($this->token(), self::NONCE);

        self::assertSame('subject-1', $claims['sub']);
    }

    public function testAKeySetWithNoSupportedKeyTypeCannotBeRead(): void
    {
        $this->expectException(UpstreamFailure::class);
        $this->expectExceptionMessage('Signing keys could not be read');

        $verifier = new OidcVerifier(
            static fn (): array => ['keys' => [['kty' => 'UNSUPPORTED', 'alg' => 'RS256', 'kid' => 'verifier-test-key']]],
            [self::ISSUER],
            self::AUDIENCE,
        );

        $verifier->verify($this->token(), self::NONCE);
    }

    /** @return array<string, string> */
    private function hmacKey(): array
    {
        return [
            'kty' => 'oct',
            'alg' => 'HS256',
            'kid' => 'hmac-test-key',
            'k' => rtrim(strtr(base64_encode('a-shared-secret-that-is-long-enough-123'), '+/', '-_'), '='),
        ];
    }

    private function verifier(): OidcVerifier
    {
        $jwks = $this->key->jwks();

        return new OidcVerifier(static fn (): array => $jwks, [self::ISSUER], self::AUDIENCE);
    }

    /** @param array<string, mixed> $overrides */
    private function token(array $overrides = []): string
    {
        return $this->key->sign(array_merge($this->claims(), $overrides));
    }

    /** @return array<string, mixed> */
    private function claims(): array
    {
        $now = time();

        return [
            'iss' => self::ISSUER,
            'aud' => self::AUDIENCE,
            'sub' => 'subject-1',
            'iat' => $now,
            'exp' => $now + 300,
            'nonce' => self::NONCE,
        ];
    }
}
