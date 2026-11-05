<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * What a provider says about the person who signed in. The email is only trusted when emailVerified is true.
 */
final readonly class ProviderIdentity
{
    public function __construct(
        public string $provider,
        public string $subject,
        public string $email,
        public bool $emailVerified,
        public ?string $name,
    ) {
    }
}
