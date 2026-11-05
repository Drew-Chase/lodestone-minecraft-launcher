<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * Reads a provider response as a JSON object. Anything else is an UpstreamFailure that names the call only.
 */
final class Responses
{
    private function __construct()
    {
    }

    /**
     * @param array{status: int, body: string} $response
     * @return array<string, mixed>
     */
    public static function object(array $response, string $label): array
    {
        if ($response['status'] !== 200) {
            throw new UpstreamFailure($label . ' returned HTTP ' . $response['status']);
        }

        $decoded = json_decode($response['body'], true);
        if (!is_array($decoded)) {
            throw new UpstreamFailure($label . ' did not return a JSON object');
        }

        return $decoded;
    }

    /** Base64url without padding, the encoding PKCE and the state values use. */
    public static function base64Url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
