<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use app\Config\Config;

/**
 * Microsoft personal accounts, through the consumers endpoint.
 */
final class MicrosoftProvider implements Provider
{
    private const AUTHORIZE_URL = 'https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize';
    private const TOKEN_URL = 'https://login.microsoftonline.com/consumers/oauth2/v2.0/token';
    private const JWKS_URL = 'https://login.microsoftonline.com/consumers/discovery/v2.0/keys';

    // The consumer tenant that personal Microsoft accounts sign in to.
    // TODO: confirm this issuer against a real Microsoft sign-in before go-live.
    private const ISSUER = 'https://login.microsoftonline.com/9188040d-6c67-4c5b-b112-36a304b66dad/v2.0';
    private const SCOPE = 'openid email profile';

    public function name(): string
    {
        return 'microsoft';
    }

    public function authorizeUrl(string $state, string $codeChallenge, string $nonce, string $redirectUri): string
    {
        $query = http_build_query([
            'client_id' => Config::need('MICROSOFT_CLIENT_ID'),
            'response_type' => 'code',
            'redirect_uri' => $redirectUri,
            'scope' => self::SCOPE,
            'state' => $state,
            'nonce' => $nonce,
            'code_challenge' => $codeChallenge,
            'code_challenge_method' => 'S256',
            'prompt' => 'select_account',
        ], '', '&', PHP_QUERY_RFC3986);

        return self::AUTHORIZE_URL . '?' . $query;
    }

    public function exchange(string $code, string $codeVerifier, string $redirectUri, Transport $transport): array
    {
        $tokens = Responses::object($transport->post(self::TOKEN_URL, [
            'grant_type' => 'authorization_code',
            'code' => $code,
            'client_id' => Config::need('MICROSOFT_CLIENT_ID'),
            'client_secret' => Config::need('MICROSOFT_CLIENT_SECRET'),
            'redirect_uri' => $redirectUri,
            'code_verifier' => $codeVerifier,
            'scope' => self::SCOPE,
        ]), 'Microsoft token exchange');

        if (!isset($tokens['id_token']) || !is_string($tokens['id_token']) || $tokens['id_token'] === '') {
            throw new \RuntimeException('Microsoft token response has no ID token');
        }

        return $tokens;
    }

    public function identity(array $tokens, string $nonce, Transport $transport): ProviderIdentity
    {
        $idToken = $tokens['id_token'] ?? null;
        if (!is_string($idToken) || $idToken === '') {
            throw new \RuntimeException('Microsoft sign-in has no ID token');
        }

        $verifier = new OidcVerifier(
            static fn (): array => Responses::object($transport->get(self::JWKS_URL), 'Microsoft key set'),
            [self::ISSUER],
            Config::need('MICROSOFT_CLIENT_ID'),
        );
        $claims = $verifier->verify($idToken, $nonce);

        $subject = $claims['sub'] ?? null;
        $email = $claims['email'] ?? null;
        if (!is_string($subject) || $subject === '' || !is_string($email) || $email === '') {
            throw new \RuntimeException('Microsoft ID token has no subject or email');
        }

        // Microsoft verifies the addresses of personal accounts, so the email claim counts as verified.
        $name = $claims['name'] ?? null;

        return new ProviderIdentity(
            'microsoft',
            $subject,
            $email,
            true,
            is_string($name) && $name !== '' ? $name : null,
        );
    }
}
