<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\AuthException;
use app\Http\Body;
use Psr\Http\Message\ServerRequestInterface;
use PHPUnit\Framework\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;

final class BodyTest extends TestCase
{
    private const JSON = '{"files": []}';

    public function test_a_body_at_the_limit_is_decoded(): void
    {
        self::assertSame(['files' => []], Body::parse($this->request(self::JSON), maxBytes: strlen(self::JSON)));
    }

    public function test_a_body_longer_than_the_limit_is_refused_as_payload_too_large(): void
    {
        try {
            Body::parse($this->request(self::JSON), maxBytes: strlen(self::JSON) - 1);
            self::fail('An oversized body was decoded.');
        } catch (AuthException $error) {
            self::assertSame('payload_too_large', $error->errorCode);
            self::assertSame(413, $error->httpStatus);
        }
    }

    private function request(string $body): ServerRequestInterface
    {
        return (new ServerRequestFactory())
            ->createServerRequest('POST', '/api/test')
            ->withBody((new StreamFactory())->createStream($body));
    }
}
