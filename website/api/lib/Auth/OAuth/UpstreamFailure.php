<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * A provider endpoint failed or answered with something unusable. The person did nothing wrong, so the callback
 * reports oauth_token rather than oauth_identity.
 */
final class UpstreamFailure extends \RuntimeException
{
}
