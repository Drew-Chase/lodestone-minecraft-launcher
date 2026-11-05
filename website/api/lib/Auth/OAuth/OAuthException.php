<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * A single sign-in failure. The code is what the login page shows, so the message is the code itself.
 * The valid codes are oauth_denied, oauth_state, oauth_token, oauth_identity and oauth_account_unverified.
 */
final class OAuthException extends \RuntimeException
{
    public function __construct(public readonly string $errorCode)
    {
        parent::__construct($errorCode);
    }
}
