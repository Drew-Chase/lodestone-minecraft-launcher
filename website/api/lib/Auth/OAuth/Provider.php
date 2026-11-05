<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

interface Provider
{
    public function name(): string;

    public function authorizeUrl(string $state, string $codeChallenge, string $nonce, string $redirectUri): string;

    /**
     * @return array<string, mixed> The decoded token response.
     * @throws \RuntimeException when the code cannot be exchanged
     */
    public function exchange(string $code, string $codeVerifier, string $redirectUri, Transport $transport): array;

    /**
     * @param array<string, mixed> $tokens The result of exchange()
     * @throws \RuntimeException when no usable, verified identity is present
     */
    public function identity(array $tokens, string $nonce, Transport $transport): ProviderIdentity;
}
