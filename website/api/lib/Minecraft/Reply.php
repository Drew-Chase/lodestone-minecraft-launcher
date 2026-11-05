<?php

declare(strict_types=1);

namespace app\Minecraft;

use app\Auth\OAuth\UpstreamFailure;

/**
 * Reads the JSON answers of the Microsoft, Xbox and Minecraft services. Failures name the call only, never the body.
 */
final class Reply
{
    private function __construct()
    {
    }

    /**
     * Requires a 2xx answer whose body is a JSON object.
     *
     * @param array{status: int, body: string} $response
     * @return array<string, mixed>
     * @throws UpstreamFailure
     */
    public static function object(array $response, string $label): array
    {
        if ($response['status'] < 200 || $response['status'] > 299) {
            throw new UpstreamFailure($label . ' returned HTTP ' . $response['status']);
        }

        $decoded = self::decode($response['body']);
        if ($decoded === null) {
            throw new UpstreamFailure($label . ' did not return a JSON object');
        }

        return $decoded;
    }

    /** @return array<string, mixed>|null */
    public static function decode(string $body): ?array
    {
        $decoded = json_decode($body, true);

        return is_array($decoded) ? $decoded : null;
    }
}
