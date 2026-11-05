<?php

declare(strict_types=1);

namespace app\Minecraft;

use app\Auth\AuthException;
use app\Auth\OAuth\Transport;
use app\Auth\OAuth\UpstreamFailure;

/**
 * Starts, polls and completes a Minecraft account link, and lists or removes the linked accounts. Every failure
 * that the HTTP layer reports is an AuthException with the code from the README. Nothing here knows about requests.
 */
final class LinkService
{
    private const SLOW_DOWN_STEP = 5;
    private const SWEEP_ONE_IN = 20;

    private readonly MicrosoftDeviceFlow $microsoft;
    private readonly XboxChain $chain;
    private readonly LinkSessions $sessions;
    private readonly MinecraftAccounts $accounts;

    public function __construct(Transport $transport)
    {
        $this->microsoft = new MicrosoftDeviceFlow($transport);
        $this->chain = new XboxChain($transport);
        $this->sessions = new LinkSessions();
        $this->accounts = new MinecraftAccounts();
    }

    /**
     * @return array{id: string, user_code: string, verification_uri: string, expires_at: string, interval: int}
     * @throws AuthException minecraft_unavailable (502)
     */
    public function start(int $userId): array
    {
        $this->sweepExpired();
        $this->sessions->deleteForUser($userId);

        try {
            $code = $this->microsoft->start();
        } catch (UpstreamFailure) {
            throw new AuthException('minecraft_unavailable', 502);
        }

        $id = bin2hex(random_bytes(16));
        $this->sessions->insert($id, $userId, $code);
        $session = $this->sessions->find($id, $userId);
        if ($session === null) {
            throw new \RuntimeException('The Minecraft link session was not stored');
        }

        return [
            'id' => $id,
            'user_code' => $code->userCode,
            'verification_uri' => $code->verificationUri,
            'expires_at' => Times::iso((string) $session['expires_at']),
            'interval' => $code->interval,
        ];
    }

    /**
     * @return array<string, mixed>
     * @throws AuthException not_found (404) when the session is missing or not the user's, slow_down (429) or
     *                       minecraft_unavailable (502)
     */
    public function status(int $userId, string $id): array
    {
        $session = $this->sessions->find($id, $userId);
        if ($session === null) {
            throw new AuthException('not_found', 404);
        }

        if ((bool) $session['expired']) {
            $this->sessions->delete($id);

            return ['status' => 'expired'];
        }

        $interval = (int) $session['interval_seconds'];
        $claim = $this->sessions->claimPoll($id, $userId);
        if ($claim === null) {
            throw new AuthException('not_found', 404);
        }
        if ($claim === false) {
            throw new AuthException('slow_down', 429, $interval);
        }

        try {
            $poll = $this->microsoft->poll((string) $session['device_code']);
        } catch (UpstreamFailure) {
            throw new AuthException('minecraft_unavailable', 502);
        }

        return match ($poll->state) {
            DevicePoll::PENDING => ['status' => 'pending', 'interval' => $interval],
            DevicePoll::SLOW_DOWN => $this->slowDown($id, $interval),
            DevicePoll::DECLINED => $this->finish($id, ['status' => 'denied']),
            DevicePoll::EXPIRED => $this->finish($id, ['status' => 'expired']),
            DevicePoll::FAILED => $this->finish($id, ['status' => 'failed', 'error' => 'minecraft_unavailable']),
            DevicePoll::AUTHORIZED => $this->complete($userId, $id, (string) $poll->accessToken),
        };
    }

    /**
     * @return array{accounts: list<array{uuid: string, name: string, linked_at: string}>}
     */
    public function accounts(int $userId): array
    {
        return ['accounts' => $this->accounts->forUser($userId)];
    }

    /** @throws AuthException not_found (404) when the user does not hold the account */
    public function unlink(int $userId, string $uuid): void
    {
        $dashed = Uuid::dashed($uuid);
        if ($dashed === null || !$this->accounts->unlink($userId, $dashed)) {
            throw new AuthException('not_found', 404);
        }
    }

    /**
     * Removes expired sessions, and with them their Microsoft device codes, at about one start in twenty. The same
     * sampling DeviceLinks uses, so no start pays for a full sweep every time.
     */
    private function sweepExpired(): void
    {
        if (random_int(1, self::SWEEP_ONE_IN) !== 1) {
            return;
        }

        $this->sessions->deleteExpired();
    }

    /** @return array<string, mixed> */
    private function slowDown(string $id, int $interval): array
    {
        $interval += self::SLOW_DOWN_STEP;
        $this->sessions->setInterval($id, $interval);

        return ['status' => 'pending', 'interval' => $interval];
    }

    /**
     * Uses Microsoft's token for the Xbox chain only, then drops it. The session ends here whatever the outcome.
     *
     * @return array<string, mixed>
     */
    private function complete(int $userId, string $id, string $microsoftToken): array
    {
        try {
            $profile = $this->chain->resolve($microsoftToken);
        } catch (NotOwned) {
            return $this->finish($id, ['status' => 'failed', 'error' => 'minecraft_not_owned']);
        } catch (Restricted) {
            return $this->finish($id, ['status' => 'failed', 'error' => 'minecraft_restricted']);
        } catch (UpstreamFailure) {
            return $this->finish($id, ['status' => 'failed', 'error' => 'minecraft_unavailable']);
        }

        $account = $this->accounts->link($userId, $profile->uuid, $profile->name);
        if ($account === null) {
            return $this->finish($id, ['status' => 'failed', 'error' => 'minecraft_linked_elsewhere']);
        }

        return $this->finish($id, ['status' => 'linked', 'account' => $account]);
    }

    /**
     * @param array<string, mixed> $result
     * @return array<string, mixed>
     */
    private function finish(string $id, array $result): array
    {
        $this->sessions->delete($id);

        return $result;
    }
}
