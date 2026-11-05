<?php

declare(strict_types=1);

namespace app\Http;

use Psr\Http\Message\ResponseInterface;

/**
 * Writes JSON bodies. Errors are `{"error": "<code>"}`, with Retry-After when the caller gives a wait.
 */
final class Json
{
    private function __construct()
    {
    }

    /** @param array<string, mixed> $data */
    public static function respond(ResponseInterface $response, array $data, int $status = 200): ResponseInterface
    {
        $response->getBody()->write(json_encode(
            $data,
            JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE
        ));

        return $response->withStatus($status)->withHeader('Content-Type', 'application/json');
    }

    public static function error(
        ResponseInterface $response,
        string $code,
        int $status,
        ?int $retryAfter = null,
    ): ResponseInterface {
        $response = self::respond($response, ['error' => $code], $status);

        if ($retryAfter !== null) {
            $response = $response->withHeader('Retry-After', (string) $retryAfter);
        }

        return $response;
    }
}
