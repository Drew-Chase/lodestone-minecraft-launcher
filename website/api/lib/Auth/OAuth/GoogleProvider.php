<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

use app\Config\Config;

final class GoogleProvider implements Provider
{
    private const AUTHORIZE_URL = 'https://accounts.google.com/o/oauth2/v2/auth';
    private const TOKEN_URL = 'https://oauth2.googleapis.com/token';
    private const JWKS_URL = 'https://www.googleapis.com/oauth2/v3/certs';
    private const ISSUERS = ['https://accounts.google.com', 'accounts.google.com'];

    public function name(): string
    {
        return 'google';
    }

    public function authorizeUrl(string $state, string $codeChallenge, string $nonce, string $redirectUri): string
    {
        $query = http_build_query([
            'client_id' => Config::need('GOOGLE_CLIENT_ID'),
            'redirect_uri' => $redirectUri,
            'response_type' => 'code',
            'scope' => 'openid email profile',
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
            'client_id' => Config::need('GOOGLE_CLIENT_ID'),
            'client_secret' => Config::need('GOOGLE_CLIENT_SECRET'),
            'redirect_uri' => $redirectUri,
            'code_verifier' => $codeVerifier,
        ]), 'Google token exchange');

        if (!isset($tokens['id_token']) || !is_string($tokens['id_token']) || $tokens['id_token'] === '') {
            throw new \RuntimeException('Google token response has no ID token');
        }

        return $tokens;
    }

    public function identity(array $tokens, string $nonce, Transport $transport): ProviderIdentity
    {
        $idToken = $tokens['id_token'] ?? null;
        if (!is_string($idToken) || $idToken === '') {
            throw new \RuntimeException('Google sign-in has no ID token');
        }

        $verifier = new OidcVerifier(
            static fn (): array => Responses::object($transport->get(self::JWKS_URL), 'Google key set'),
            self::ISSUERS,
            Config::need('GOOGLE_CLIENT_ID'),
        );
        $claims = $verifier->verify($idToken, $nonce);

        $subject = $claims['sub'] ?? null;
        $email = $claims['email'] ?? null;
        if (!is_string($subject) || $subject === '' || !is_string($email) || $email === '') {
            throw new \RuntimeException('Google ID token has no subject or email');
        }

        // Google sends the flag as a boolean. Some responses send the string form instead.
        $verified = $claims['email_verified'] ?? null;
        $emailVerified = $verified === true || $verified === 'true';

        $name = $claims['name'] ?? null;

        return new ProviderIdentity(
            'google',
            $subject,
            $email,
            $emailVerified,
            is_string($name) && $name !== '' ? $name : null,
        );
    }
}
