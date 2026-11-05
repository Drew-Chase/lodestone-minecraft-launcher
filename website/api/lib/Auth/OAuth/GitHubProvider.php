<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use app\Config\Config;

/**
 * GitHub OAuth. GitHub takes neither PKCE nor a nonce, so neither is sent. The verified address comes from /user/emails.
 */
final class GitHubProvider implements Provider
{
    private const AUTHORIZE_URL = 'https://github.com/login/oauth/authorize';
    private const TOKEN_URL = 'https://github.com/login/oauth/access_token';
    private const USER_URL = 'https://api.github.com/user';
    private const EMAILS_URL = 'https://api.github.com/user/emails';
    private const SCOPE = 'read:user user:email';

    public function name(): string
    {
        return 'github';
    }

    public function authorizeUrl(string $state, string $codeChallenge, string $nonce, string $redirectUri): string
    {
        $query = http_build_query([
            'client_id' => Config::need('GITHUB_CLIENT_ID'),
            'redirect_uri' => $redirectUri,
            'scope' => self::SCOPE,
            'state' => $state,
        ], '', '&', PHP_QUERY_RFC3986);

        return self::AUTHORIZE_URL . '?' . $query;
    }

    public function exchange(string $code, string $codeVerifier, string $redirectUri, Transport $transport): array
    {
        $tokens = Responses::object($transport->post(
            self::TOKEN_URL,
            [
                'client_id' => Config::need('GITHUB_CLIENT_ID'),
                'client_secret' => Config::need('GITHUB_CLIENT_SECRET'),
                'code' => $code,
                'redirect_uri' => $redirectUri,
            ],
            ['Accept' => 'application/json'],
        ), 'GitHub token exchange');

        // GitHub answers a bad code with a 200 and an error field, so the access token is checked here.
        if (!isset($tokens['access_token']) || !is_string($tokens['access_token']) || $tokens['access_token'] === '') {
            throw new \RuntimeException('GitHub token response has no access token');
        }

        return $tokens;
    }

    public function identity(array $tokens, string $nonce, Transport $transport): ProviderIdentity
    {
        $accessToken = $tokens['access_token'] ?? null;
        if (!is_string($accessToken) || $accessToken === '') {
            throw new \RuntimeException('GitHub sign-in has no access token');
        }

        $headers = [
            'Authorization' => 'Bearer ' . $accessToken,
            'Accept' => 'application/vnd.github+json',
        ];

        $user = Responses::object($transport->get(self::USER_URL, $headers), 'GitHub user');

        $id = $user['id'] ?? null;
        if (!is_int($id) && !(is_string($id) && ctype_digit($id) && $id !== '')) {
            throw new \RuntimeException('GitHub user has no id');
        }

        $email = $this->verifiedPrimaryEmail(
            Responses::object($transport->get(self::EMAILS_URL, $headers), 'GitHub emails'),
        );

        $name = $user['name'] ?? null;

        return new ProviderIdentity(
            'github',
            (string) $id,
            $email,
            true,
            is_string($name) && $name !== '' ? $name : null,
        );
    }

    /**
     * @param array<mixed> $emails The entries of /user/emails.
     * @throws \RuntimeException when no entry is both primary and verified
     */
    private function verifiedPrimaryEmail(array $emails): string
    {
        foreach ($emails as $entry) {
            if (!is_array($entry)) {
                continue;
            }

            $address = $entry['email'] ?? null;
            if (($entry['primary'] ?? null) === true
                && ($entry['verified'] ?? null) === true
                && is_string($address)
                && $address !== ''
            ) {
                return $address;
            }
        }

        throw new \RuntimeException('GitHub account has no primary verified email');
    }
}
