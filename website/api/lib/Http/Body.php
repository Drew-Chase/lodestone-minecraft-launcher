<?php

declare(strict_types=1);

namespace app\Http;

use app\Auth\AuthException;
use JsonException;
use Psr\Http\Message\ServerRequestInterface;
use Psr\Http\Message\StreamInterface;

final class Body
{
    private const DEPTH = 512;
    private const CHUNK = 65536;

    private function __construct()
    {
    }

    /**
     * Decodes a JSON body. An empty body is allowed only when $optional is true. When $maxBytes is set, the body is read in
     * pieces and refused as soon as it is longer than that, so an oversized body is never held whole in memory.
     *
     * @return array<string, mixed>
     * @throws AuthException invalid_json (400), payload_too_large (413)
     */
    public static function parse(ServerRequestInterface $request, bool $optional = false, ?int $maxBytes = null): array
    {
        $raw = $maxBytes === null
            ? (string) $request->getBody()
            : self::bounded($request->getBody(), $maxBytes);

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

    /**
     * Reads the whole body, stopping as soon as it passes the limit.
     *
     * @throws AuthException payload_too_large (413)
     */
    private static function bounded(StreamInterface $stream, int $maxBytes): string
    {
        if ($stream->isSeekable()) {
            $stream->rewind();
        }

        $raw = '';
        while (!$stream->eof()) {
            $chunk = $stream->read(self::CHUNK);

            // An empty read is the end of the body, even where eof() has not reported it yet.
            if ($chunk === '') {
                break;
            }

            $raw .= $chunk;
            if (strlen($raw) > $maxBytes) {
                throw new AuthException('payload_too_large', 413);
            }
        }

        return $raw;
    }
}
