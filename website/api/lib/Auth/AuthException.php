<?php

declare(strict_types=1);

namespace app\Auth;

/**
 * A failure the HTTP layer turns into `{"error": $errorCode}` with $httpStatus.
 * The message is the error code, so it never carries a token or an address.
 */
final class AuthException extends \RuntimeException
{
    public function __construct(
        public readonly string $errorCode,
        public readonly int $httpStatus,
        public readonly ?int $retryAfter = null,
    ) {
        parent::__construct($errorCode);
    }
}
