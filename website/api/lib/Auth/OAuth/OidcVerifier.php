<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use Firebase\JWT\JWK;
use Firebase\JWT\JWT;
use Firebase\JWT\Key;

/**
 * Checks an OpenID Connect ID token: RS256 signature against the published keys, issuer, audience, expiry and nonce.
 * Every failure is a \RuntimeException whose message names the rule, never the token.
 */
final class OidcVerifier
{
    private const ALGORITHM = 'RS256';

    /**
     * @param \Closure(): array<string, mixed> $jwks Returns the provider's JWK Set.
     * @param list<string> $issuers
     */
    public function __construct(
        private readonly \Closure $jwks,
        private readonly array $issuers,
        private readonly string $audience,
    ) {
    }

    /**
     * @return array<string, mixed> The verified claims.
     * @throws \RuntimeException when any check fails
     */
    public function verify(string $idToken, string $nonce): array
    {
        if ($this->audience === '' || $nonce === '') {
            throw new \RuntimeException('ID token check has no audience or nonce');
        }

        $keys = $this->signingKeys();

        try {
            $claims = (array) JWT::decode($idToken, $keys);
        } catch (\Throwable) {
            // The library's message can echo parts of the token, so it is dropped.
            throw new \RuntimeException('ID token signature or timing check failed');
        }

        $issuer = $claims['iss'] ?? null;
        if (!is_string($issuer) || !in_array($issuer, $this->issuers, true)) {
            throw new \RuntimeException('ID token issuer is not accepted');
        }

        $audience = $claims['aud'] ?? null;
        if (!is_string($audience) || !hash_equals($this->audience, $audience)) {
            throw new \RuntimeException('ID token audience is not this client');
        }

        $expires = $claims['exp'] ?? null;
        if (!is_numeric($expires) || (float) $expires <= time()) {
            throw new \RuntimeException('ID token has expired');
        }

        $tokenNonce = $claims['nonce'] ?? null;
        if (!is_string($tokenNonce) || !hash_equals($nonce, $tokenNonce)) {
            throw new \RuntimeException('ID token nonce does not match');
        }

        return $claims;
    }

    /**
     * Only RS256 keys are used, so a key the set labels with another algorithm can never verify a token.
     *
     * @return array<string, Key>
     */
    private function signingKeys(): array
    {
        // The key set comes from the provider, so a failure to read it is the provider's, not the person's.
        try {
            $set = ($this->jwks)();
        } catch (\Throwable) {
            throw new UpstreamFailure('Signing keys could not be read');
        }

        // The library refuses an empty set with its own error, so an empty set is reported as the missing key it is.
        if (($set['keys'] ?? null) === []) {
            throw new UpstreamFailure('No RS256 signing key is published');
        }

        try {
            $parsed = JWK::parseKeySet($set, self::ALGORITHM);
        } catch (\Throwable) {
            throw new UpstreamFailure('Signing keys could not be read');
        }

        $keys = array_filter(
            $parsed,
            static fn (Key $key): bool => $key->getAlgorithm() === self::ALGORITHM
        );

        if ($keys === []) {
            throw new UpstreamFailure('No RS256 signing key is published');
        }

        return $keys;
    }
}
