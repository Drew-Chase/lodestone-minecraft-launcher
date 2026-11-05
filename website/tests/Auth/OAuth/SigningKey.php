<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use Firebase\JWT\JWT;

/**
 * A throwaway RSA key for signing test ID tokens. It is generated in memory and never stored.
 */
final class SigningKey
{
    private function __construct(
        private readonly string $kid,
        private readonly string $privatePem,
    ) {
    }

    private const KEY_PARAMS = ['private_key_bits' => 2048, 'private_key_type' => OPENSSL_KEYTYPE_RSA];

    public static function generate(string $kid): self
    {
        $options = [];
        $key = openssl_pkey_new(self::KEY_PARAMS);

        // Windows PHP builds keep a copy of the OpenSSL config beside php.exe. Use it when the default config is missing.
        if ($key === false) {
            $bundled = dirname(PHP_BINARY) . '/extras/ssl/openssl.cnf';
            if (is_file($bundled)) {
                $options = ['config' => $bundled];
                $key = openssl_pkey_new(self::KEY_PARAMS + $options);
            }
        }

        if ($key === false) {
            throw new \RuntimeException('The test key could not be generated');
        }

        if (!openssl_pkey_export($key, $pem, null, $options)) {
            throw new \RuntimeException('The test key could not be exported');
        }

        return new self($kid, $pem);
    }

    /** @return array{keys: list<array<string, string>>} The public half, as a JWK Set. */
    public function jwks(): array
    {
        $details = openssl_pkey_get_details(openssl_pkey_get_private($this->privatePem));
        if ($details === false || !isset($details['rsa'])) {
            throw new \RuntimeException('The test key has no RSA details');
        }

        return [
            'keys' => [[
                'kty' => 'RSA',
                'kid' => $this->kid,
                'alg' => 'RS256',
                'use' => 'sig',
                'n' => self::base64Url($details['rsa']['n']),
                'e' => self::base64Url($details['rsa']['e']),
            ]],
        ];
    }

    public function publicPem(): string
    {
        $details = openssl_pkey_get_details(openssl_pkey_get_private($this->privatePem));
        if ($details === false || !isset($details['key'])) {
            throw new \RuntimeException('The test key has no public PEM');
        }

        return $details['key'];
    }

    /** @param array<string, mixed> $claims */
    public function sign(array $claims): string
    {
        return JWT::encode($claims, $this->privatePem, 'RS256', $this->kid);
    }

    private static function base64Url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
