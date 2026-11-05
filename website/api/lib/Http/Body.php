<?php

declare(strict_types=1);

namespace app\Http;

use app\Auth\AuthException;
use JsonException;
use Psr\Http\Message\ServerRequestInterface;

final class Body
{
    private const DEPTH = 512;

    private function __construct()
    {
    }

    /**
     * Decodes a JSON body. An empty body is allowed only when $optional is true.
     *
     * @return array<string, mixed>
     * @throws AuthException invalid_json (400)
     */
    public static function parse(ServerRequestInterface $request, bool $optional = false): array
    {
        $raw = (string) $request->getBody();

        if (trim($raw) === '') {
            if ($optional) {
                return [];
            }

            throw new AuthException('invalid_json', 400);
        }

        try {
            $decoded = json_decode($raw, false, self::DEPTH, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            throw new AuthException('invalid_json', 400);
        }

        // Objects decode to stdClass, so a JSON array such as [] is rejected here.
        if (!is_object($decoded)) {
            throw new AuthException('invalid_json', 400);
        }

        return get_object_vars($decoded);
    }
}
