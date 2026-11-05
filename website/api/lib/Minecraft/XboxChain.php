<?php

declare(strict_types=1);

namespace app\Minecraft;

use app\Auth\OAuth\Transport;
use app\Auth\OAuth\UpstreamFailure;

/**
 * Turns a Microsoft access token into a Minecraft profile: Xbox Live user token, XSTS token, Minecraft services
 * login, then the profile. Each call follows the minecraft-msa-auth crate the launcher uses.
 */
final class XboxChain
{
    private const XBOX_USER_URL = 'https://user.auth.xboxlive.com/user/authenticate';
    private const XSTS_URL = 'https://xsts.auth.xboxlive.com/xsts/authorize';
    private const MINECRAFT_LOGIN_URL = 'https://api.minecraftservices.com/authentication/login_with_xbox';
    private const PROFILE_URL = 'https://api.minecraftservices.com/minecraft/profile';
    private const HEADERS = ['Accept' => 'application/json'];
    private const NO_XBOX_ACCOUNT = 2148916233;

    /** Country, region, adult-verification and family refusals from DESIGN.md section 6. */
    private const RESTRICTED = [2148916234, 2148916235, 2148916236, 2148916237, 2148916238];

    /** Minecraft names are one to sixteen letters, digits or underscores. Anything else is not stored. */
    private const NAME_PATTERN = '/^[A-Za-z0-9_]{1,16}$/D';

    public function __construct(private readonly Transport $transport)
    {
    }

    /**
     * @throws NotOwned when the account cannot play Minecraft
     * @throws Restricted when Xbox Live does not allow the account in its country or region, or needs adult checks
     * @throws UpstreamFailure when a service fails or answers unusably
     */
    public function resolve(string $microsoftToken): MinecraftProfile
    {
        [$xboxToken, $userHash] = $this->xboxUserToken($microsoftToken);
        $xstsToken = $this->xstsToken($xboxToken);
        $minecraftToken = $this->minecraftToken($userHash, $xstsToken);

        return $this->profile($minecraftToken);
    }

    /** @return array{0: string, 1: string} The Xbox token and the user hash. */
    private function xboxUserToken(string $microsoftToken): array
    {
        $body = Reply::object($this->transport->postJson(self::XBOX_USER_URL, [
            'Properties' => [
                'AuthMethod' => 'RPS',
                'SiteName' => 'user.auth.xboxlive.com',
                'RpsTicket' => 'd=' . $microsoftToken,
            ],
            'RelyingParty' => 'http://auth.xboxlive.com',
            'TokenType' => 'JWT',
        ], self::HEADERS), 'Xbox Live sign-in');

        $token = $body['Token'] ?? null;
        $userHash = $body['DisplayClaims']['xui'][0]['uhs'] ?? null;
        if (!is_string($token) || $token === '' || !is_string($userHash) || $userHash === '') {
            throw new UpstreamFailure('Xbox Live sign-in has no token or user hash');
        }

        return [$token, $userHash];
    }

    private function xstsToken(string $xboxToken): string
    {
        $response = $this->transport->postJson(self::XSTS_URL, [
            'Properties' => [
                'SandboxId' => 'RETAIL',
                'UserTokens' => [$xboxToken],
            ],
            'RelyingParty' => 'rp://api.minecraftservices.com/',
            'TokenType' => 'JWT',
        ], self::HEADERS);

        // Xbox Live names its refusal in XErr whatever the status, so the code is read before the status is judged.
        $code = self::xerr(Reply::decode($response['body']) ?? []);
        if ($code === self::NO_XBOX_ACCOUNT) {
            throw new NotOwned('Xbox Live refused the account');
        }
        if ($code !== null && in_array($code, self::RESTRICTED, true)) {
            throw new Restricted('Xbox Live restricts the account');
        }

        if ($response['status'] === 401) {
            throw new UpstreamFailure('Xbox Live refused the sign-in');
        }

        $body = Reply::object($response, 'XSTS authorization');
        $token = $body['Token'] ?? null;
        if (!is_string($token) || $token === '') {
            throw new UpstreamFailure('XSTS authorization has no token');
        }

        return $token;
    }

    /** The XErr code as an integer. Xbox Live sends it as a number or as a numeric string. Null when there is none. */
    private static function xerr(array $body): ?int
    {
        $code = $body['XErr'] ?? null;

        if (is_int($code)) {
            return $code;
        }

        return is_string($code) && preg_match('/^[0-9]{1,18}$/D', $code) === 1 ? (int) $code : null;
    }

    private function minecraftToken(string $userHash, string $xstsToken): string
    {
        $body = Reply::object($this->transport->postJson(self::MINECRAFT_LOGIN_URL, [
            'identityToken' => 'XBL3.0 x=' . $userHash . ';' . $xstsToken,
        ], self::HEADERS), 'Minecraft sign-in');

        $token = $body['access_token'] ?? null;
        if (!is_string($token) || $token === '') {
            throw new UpstreamFailure('Minecraft sign-in has no access token');
        }

        return $token;
    }

    private function profile(string $minecraftToken): MinecraftProfile
    {
        $response = $this->transport->get(self::PROFILE_URL, ['Authorization' => 'Bearer ' . $minecraftToken]);
        if ($response['status'] === 404) {
            throw new NotOwned('The account has no Minecraft profile');
        }

        $body = Reply::object($response, 'Minecraft profile');
        $uuid = is_string($body['id'] ?? null) ? Uuid::dashed($body['id']) : null;
        $name = $body['name'] ?? null;
        if ($uuid === null || !is_string($name) || preg_match(self::NAME_PATTERN, $name) !== 1) {
            throw new UpstreamFailure('Minecraft profile is incomplete or its name is not a Minecraft name');
        }

        return new MinecraftProfile($uuid, $name);
    }
}
