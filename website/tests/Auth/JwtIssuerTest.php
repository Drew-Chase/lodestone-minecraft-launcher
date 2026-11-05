<?php

declare(strict_types=1);

namespace tests\Auth;

use app\Auth\AuthException;
use app\Auth\JwtIssuer;
use Firebase\JWT\JWT;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use RuntimeException;

final class JwtIssuerTest extends TestCase
{
    private const ISSUER = 'https://issuer.test';
    private const FAMILY = '0123456789abcdef0123456789abcdef';

    private string $secret;

    protected function setUp(): void
    {
        parent::setUp();

        $this->secret = str_repeat('k', 32);
    }

    public function testShortSecretIsRejectedWithoutEchoingIt(): void
    {
        $short = 'tiny-secret-value';

        try {
            new JwtIssuer($short, self::ISSUER);
            self::fail('A secret under 32 bytes must be rejected.');
        } catch (RuntimeException $error) {
            self::assertStringContainsString('32 bytes', $error->getMessage());
            self::assertStringNotContainsString($short, $error->getMessage());
        }
    }

    public function testIssueThenVerifyReturnsUserAndFamily(): void
    {
        $jwt = $this->issuer()->issue(42, self::FAMILY);

        $claims = $this->issuer()->verify($jwt['token']);

        self::assertSame(42, $claims['user_id']);
        self::assertSame(self::FAMILY, $claims['family_id']);
    }

    public function testIssueReportsFifteenMinutes(): void
    {
        $jwt = $this->issuer()->issue(1, self::FAMILY);

        self::assertSame(900, $jwt['expires_in']);
    }

    public function testTokenSignedWithAnotherSecretIsRejected(): void
    {
        $forged = JWT::encode($this->claims(), str_repeat('z', 32), 'HS256');

        $this->assertUnauthorized($forged);
    }

    public function testTokenSignedWithHs512IsRejected(): void
    {
        // HS512 needs a 64-byte key to sign. The verifier must reject it on the algorithm alone.
        $token = JWT::encode($this->claims(), str_repeat('k', 64), 'HS512');

        $this->assertUnauthorized($token);
    }

    public function testTokenWithAnotherIssuerIsRejected(): void
    {
        $token = JWT::encode(['iss' => 'https://evil.test'] + $this->claims(), $this->secret, 'HS256');

        $this->assertUnauthorized($token);
    }

    public function testTokenWithAnotherTypeIsRejected(): void
    {
        $token = JWT::encode(['typ' => 'refresh'] + $this->claims(), $this->secret, 'HS256');

        $this->assertUnauthorized($token);
    }

    public function testExpiredTokenIsRejected(): void
    {
        $token = JWT::encode(['exp' => time() - 10] + $this->claims(), $this->secret, 'HS256');

        $this->assertUnauthorized($token);
    }

    /** @return iterable<string, array{mixed}> */
    public static function badSubjects(): iterable
    {
        yield 'zero' => ['0'];
        yield 'leading zero' => ['01'];
        yield 'letters' => ['abc'];
        yield 'negative' => ['-3'];
        yield 'integer not string' => [12];
    }

    #[DataProvider('badSubjects')]
    public function testSubjectThatIsNotAPositiveIntegerStringIsRejected(mixed $subject): void
    {
        $token = JWT::encode(['sub' => $subject] + $this->claims(), $this->secret, 'HS256');

        $this->assertUnauthorized($token);
    }

    public function testGarbageTokenIsRejected(): void
    {
        $this->assertUnauthorized('not.a.jwt');
    }

    /** @return array<string, mixed> */
    private function claims(): array
    {
        $now = time();

        return [
            'iss' => self::ISSUER,
            'sub' => '7',
            'iat' => $now,
            'nbf' => $now,
            'exp' => $now + 900,
            'typ' => 'access',
            'fid' => self::FAMILY,
        ];
    }

    private function issuer(): JwtIssuer
    {
        return new JwtIssuer($this->secret, self::ISSUER);
    }

    private function assertUnauthorized(string $token): void
    {
        try {
            $this->issuer()->verify($token);
            self::fail('The token must be rejected.');
        } catch (AuthException $error) {
            self::assertSame('unauthorized', $error->errorCode);
            self::assertSame(401, $error->httpStatus);
        }
    }
}
