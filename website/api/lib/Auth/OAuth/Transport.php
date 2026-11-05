<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * The outbound HTTP calls a provider makes. Headers are name => value. Every method returns
 * ['status' => int, 'body' => string] and throws \RuntimeException only when no response arrives.
 */
interface Transport
{
    /**
     * @param array<string, string> $form
     * @param array<string, string> $headers
     * @return array{status: int, body: string}
     */
    public function post(string $url, array $form, array $headers = []): array;

    /**
     * Sends $json as the body, with Content-Type application/json unless $headers sets one.
     *
     * @param array<string, mixed> $json
     * @param array<string, string> $headers
     * @return array{status: int, body: string}
     */
    public function postJson(string $url, array $json, array $headers = []): array;

    /**
     * @param array<string, string> $headers
     * @return array{status: int, body: string}
     */
    public function get(string $url, array $headers = []): array;
}
