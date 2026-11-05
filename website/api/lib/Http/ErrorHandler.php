<?php

declare(strict_types=1);

namespace app\Http;

use app\Auth\AuthException;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\Exception\HttpException;
use Slim\Psr7\Response;
use Throwable;

/**
 * Turns every failure into a JSON error. Exception messages never reach the response, and
 * unexpected errors are written to the server log by class and message only.
 */
final class ErrorHandler
{
    private function __construct()
    {
    }

    public static function handle(
        ServerRequestInterface $request,
        Throwable $exception,
        bool $displayErrorDetails = false,
        bool $logErrors = false,
        bool $logErrorDetails = false,
    ): ResponseInterface {
        $response = new Response();

        if ($exception instanceof AuthException) {
            return Json::error($response, $exception->errorCode, $exception->httpStatus, $exception->retryAfter);
        }

        if ($exception instanceof HttpException) {
            $status = $exception->getCode();
            if ($status < 400 || $status > 599) {
                $status = 500;
            }

            $code = match ($status) {
                404 => 'not_found',
                405 => 'method_not_allowed',
                default => 'http_error',
            };

            return Json::error($response, $code, $status);
        }

        error_log('Unhandled ' . $exception::class . ': ' . $exception->getMessage());

        return Json::error($response, 'internal_error', 500);
    }
}
