<?php

declare(strict_types=1);

namespace app\Minecraft;

use app\Auth\OAuth\Transport;
use app\Auth\OAuth\UpstreamFailure;

/**
 * The Microsoft device-code grant for the launcher's public app: the request for a code the person enters,
 * and the poll that waits for them to enter it. Consumer accounts only.
 */
final class MicrosoftDeviceFlow
{
    private const CLIENT_ID = 'b1156b7e-e0fe-4f86-a835-afed728d093d';
    private const DEVICE_CODE_URL = 'https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode';
    private const TOKEN_URL = 'https://login.microsoftonline.com/consumers/oauth2/v2.0/token';
    private const SCOPE = 'XboxLive.signin offline_access';
    private const GRANT_TYPE = 'urn:ietf:params:oauth:grant-type:device_code';
    private const DEFAULT_INTERVAL = 5;
    private const HEADERS = ['Accept' => 'application/json'];

    /** Client errors that mean the sign-in can never succeed. Any other unknown 400 is retried by the next poll. */
    private const FAILED_ERRORS = ['bad_verification_code', 'invalid_grant'];

    public function __construct(private readonly Transport $transport)
    {
    }

    /** @throws UpstreamFailure */
    public function start(): DeviceCode
    {
        $body = Reply::object($this->transport->post(self::DEVICE_CODE_URL, [
            'client_id' => self::CLIENT_ID,
            'scope' => self::SCOPE,
        ], self::HEADERS), 'Microsoft device code');

        $deviceCode = self::text($body['device_code'] ?? null, 512);
        $userCode = self::text($body['user_code'] ?? null, 16);
        $verificationUri = self::text($body['verification_uri'] ?? null, 255);
        $expiresIn = $body['expires_in'] ?? null;
        $interval = $body['interval'] ?? self::DEFAULT_INTERVAL;

        if ($deviceCode === null || $userCode === null || $verificationUri === null
            || !is_int($expiresIn) || $expiresIn < 1 || !is_int($interval) || $interval < 1) {
            throw new UpstreamFailure('Microsoft device code response is incomplete');
        }

        return new DeviceCode($deviceCode, $userCode, $verificationUri, $expiresIn, $interval);
    }

    /**
     * A 400 whose error is bad_verification_code or invalid_grant is FAILED, so the session ends. Any other unexpected
     * answer, such as an unknown 400, a network error or a 5xx, is an UpstreamFailure and the session is kept for the
     * next poll.
     *
     * @throws UpstreamFailure when the answer is a token-less success, an unknown error, or a server-side failure
     */
    public function poll(string $deviceCode): DevicePoll
    {
        $response = $this->transport->post(self::TOKEN_URL, [
            'client_id' => self::CLIENT_ID,
            'grant_type' => self::GRANT_TYPE,
            'device_code' => $deviceCode,
        ], self::HEADERS);

        $body = Reply::decode($response['body']);

        if ($response['status'] === 200) {
            if ($body === null) {
                throw new UpstreamFailure('Microsoft token poll did not return a JSON object');
            }

            $accessToken = self::text($body['access_token'] ?? null, PHP_INT_MAX);
            if ($accessToken === null) {
                throw new UpstreamFailure('Microsoft token response has no access token');
            }

            return new DevicePoll(DevicePoll::AUTHORIZED, $accessToken);
        }

        $error = $body['error'] ?? null;

        return match ($error) {
            'authorization_pending' => new DevicePoll(DevicePoll::PENDING),
            'slow_down' => new DevicePoll(DevicePoll::SLOW_DOWN),
            'authorization_declined', 'access_denied' => new DevicePoll(DevicePoll::DECLINED),
            'expired_token' => new DevicePoll(DevicePoll::EXPIRED),
            default => $response['status'] === 400 && in_array($error, self::FAILED_ERRORS, true)
                ? new DevicePoll(DevicePoll::FAILED)
                : throw new UpstreamFailure('Microsoft token poll failed'),
        };
    }

    private static function text(mixed $value, int $maxLength): ?string
    {
        return is_string($value) && $value !== '' && strlen($value) <= $maxLength ? $value : null;
    }
}
