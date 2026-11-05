<?php

declare(strict_types=1);

namespace app\Auth;

use app\Config\Config;
use Firebase\JWT\JWT;
use Firebase\JWT\Key;

/**
 * Issues and checks HS256 access tokens. Access tokens are stateless, so a revoked session
 * keeps working until its token expires (at most TTL_SECONDS).
 */
final class JwtIssuer
{
    public const TTL_SECONDS = 900;

    private const ALGORITHM = 'HS256';
    private const MIN_SECRET_BYTES = 32;

    public function __construct(
        private readonly string $secret,
        private readonly string $issuer,
    ) {
        // The message names the rule, never the secret.
        if (strlen($secret) < self::MIN_SECRET_BYTES) {
            throw new \RuntimeException('JWT_SECRET must be at least ' . self::MIN_SECRET_BYTES . ' bytes');
        }
    }

    public static function fromConfig(): self
    {
        $issuer = rtrim(Config::need('APP_URL'), '/');

        return new self(Config::need('JWT_SECRET'), $issuer);
    }

    /**
     * @return array{token: string, expires_in: int}
     */
    public function issue(int $userId, string $familyId): array
    {
        $now = time();
        $payload = [
            'iss' => $this->issuer,
            'sub' => (string) $userId,
            'iat' => $now,
            'nbf' => $now,
            'exp' => $now + self::TTL_SECONDS,
            'typ' => 'access',
            'fid' => $familyId,
        ];

        return [
            'token' => JWT::encode($payload, $this->secret, self::ALGORITHM),
            'expires_in' => self::TTL_SECONDS,
        ];
    }

    /**
     * @return array{user_id: int, family_id: string}
     * @throws AuthException unauthorized (401) for any failure
     */
    public function verify(string $jwt): array
    {
        try {
            $headers = new \stdClass();
            $claims = JWT::decode($jwt, new Key($this->secret, self::ALGORITHM), $headers);
        } catch (\Throwable) {
            // The library's message can echo parts of the token, so it is dropped.
            throw new AuthException('unauthorized', 401);
        }

        if (($headers->alg ?? null) !== self::ALGORITHM) {
            throw new AuthException('unauthorized', 401);
        }

        $issuer = $claims->iss ?? null;
        $type = $claims->typ ?? null;
        $subject = $claims->sub ?? null;
        $family = $claims->fid ?? null;
        $expires = $claims->exp ?? null;

        if ($issuer !== $this->issuer || $type !== 'access' || $expires === null) {
            throw new AuthException('unauthorized', 401);
        }

        if (!is_string($subject) || preg_match('/^[1-9][0-9]*$/D', $subject) !== 1) {
            throw new AuthException('unauthorized', 401);
        }

        $userId = (int) $subject;
        if ((string) $userId !== $subject) {
            throw new AuthException('unauthorized', 401);
        }

        if (!is_string($family) || $family === '') {
            throw new AuthException('unauthorized', 401);
        }

        return ['user_id' => $userId, 'family_id' => $family];
    }
}
