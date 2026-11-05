<?php

declare(strict_types=1);

namespace app\Http;

use app\Auth\AuthException;
use app\Auth\JwtIssuer;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface;

/**
 * Checks `Authorization: Bearer <access token>` and adds the user_id (int) and family_id (string) attributes.
 * Failures throw, so the error handler writes the response.
 */
final class BearerAuth implements MiddlewareInterface
{
    private const PREFIX = 'Bearer ';

    public function process(ServerRequestInterface $request, RequestHandlerInterface $handler): ResponseInterface
    {
        $header = $request->getHeaderLine('Authorization');

        if (!str_starts_with($header, self::PREFIX)) {
            throw new AuthException('unauthorized', 401);
        }

        $token = substr($header, strlen(self::PREFIX));
        if ($token === '' || str_contains($token, ' ')) {
            throw new AuthException('unauthorized', 401);
        }

        $claims = JwtIssuer::fromConfig()->verify($token);

        $request = $request
            ->withAttribute('user_id', $claims['user_id'])
            ->withAttribute('family_id', $claims['family_id']);

        return $handler->handle($request);
    }
}
