<?php

declare(strict_types=1);

namespace tests\Minecraft;

use app\Auth\OAuth\Transport;
use app\Auth\OAuth\Transports;
use app\Minecraft\LinkService;
use app\Users\UserRepository;
use PDO;
use tests\Auth\OAuth\FakeTransport;

/**
 * Canned Microsoft, Xbox and Minecraft answers for the link tests. The fake transport is installed per test, and the
 * transport that was current before is put back afterwards, as the OAuth tests do. No test reaches the network.
 */
trait LinkFixtures
{
    protected const MS_CLIENT_ID = 'b1156b7e-e0fe-4f86-a835-afed728d093d';
    protected const DEVICE_URL = 'https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode';
    protected const TOKEN_URL = 'https://login.microsoftonline.com/consumers/oauth2/v2.0/token';
    protected const XBOX_USER_URL = 'https://user.auth.xboxlive.com/user/authenticate';
    protected const XSTS_URL = 'https://xsts.auth.xboxlive.com/xsts/authorize';
    protected const MINECRAFT_LOGIN_URL = 'https://api.minecraftservices.com/authentication/login_with_xbox';
    protected const PROFILE_URL = 'https://api.minecraftservices.com/minecraft/profile';

    protected const DEVICE_CODE = 'device-code-for-tests';
    protected const USER_CODE = 'ABCD1234';
    protected const MICROSOFT_TOKEN = 'microsoft-access-token-for-tests';
    protected const XBOX_TOKEN = 'xbox-user-token-for-tests';
    protected const USER_HASH = 'user-hash-for-tests';
    protected const XSTS_TOKEN = 'xsts-token-for-tests';
    protected const MINECRAFT_TOKEN = 'minecraft-token-for-tests';
    protected const UUID_HEX = '0123abcd0123abcd0123abcd0123abcd';
    protected const UUID = '0123abcd-0123-abcd-0123-abcd0123abcd';
    protected const NAME = 'Steve';

    protected FakeTransport $transport;
    private ?Transport $previousTransport = null;

    protected function installTransport(): void
    {
        $this->previousTransport = Transports::current();
        $this->transport = new FakeTransport();
        Transports::use($this->transport);
    }

    protected function restoreTransport(): void
    {
        if ($this->previousTransport !== null) {
            Transports::use($this->previousTransport);
        }
    }

    protected function answerDeviceCode(): void
    {
        $this->transport->respondWith(self::DEVICE_URL, FakeTransport::json([
            'device_code' => self::DEVICE_CODE,
            'user_code' => self::USER_CODE,
            'verification_uri' => 'https://www.microsoft.com/link',
            'expires_in' => 900,
            'interval' => 5,
        ]));
    }

    /** @param array<string, mixed> $body */
    protected function answerPoll(array $body, int $status = 400): void
    {
        $this->transport->respondWith(self::TOKEN_URL, FakeTransport::json($body, $status));
    }

    /** Microsoft grants the token, and every Xbox, Minecraft and profile call succeeds for an owned account. */
    protected function answerOwnedChain(): void
    {
        $this->answerPoll(['access_token' => self::MICROSOFT_TOKEN], 200);

        $this->transport->respondWith(self::XBOX_USER_URL, FakeTransport::json([
            'Token' => self::XBOX_TOKEN,
            'DisplayClaims' => ['xui' => [['uhs' => self::USER_HASH]]],
        ]));
        $this->transport->respondWith(self::XSTS_URL, FakeTransport::json([
            'Token' => self::XSTS_TOKEN,
            'DisplayClaims' => ['xui' => [['uhs' => self::USER_HASH]]],
        ]));
        $this->transport->respondWith(self::MINECRAFT_LOGIN_URL, FakeTransport::json([
            'username' => self::USER_HASH,
            'access_token' => self::MINECRAFT_TOKEN,
            'token_type' => 'Bearer',
            'expires_in' => 86400,
        ]));
        $this->transport->respondWith(self::PROFILE_URL, FakeTransport::json([
            'id' => self::UUID_HEX,
            'name' => self::NAME,
            'skins' => [],
            'capes' => [],
        ]));
    }

    protected function createUser(string $email): int
    {
        return (new UserRepository())->create($email, null, null);
    }

    protected function startLink(LinkService $service, int $userId): string
    {
        $this->answerDeviceCode();

        return $service->start($userId)['id'];
    }

    /**
     * @param list<mixed> $params
     * @return list<array<string, mixed>>
     */
    protected function rows(string $sql, array $params = []): array
    {
        $statement = $this->pdo()->prepare($sql);
        $statement->execute($params);

        return $statement->fetchAll(PDO::FETCH_ASSOC);
    }

    /** @param list<mixed> $params */
    protected function scalar(string $sql, array $params = []): int
    {
        $statement = $this->pdo()->prepare($sql);
        $statement->execute($params);

        return (int) $statement->fetchColumn();
    }

    /** @return list<array{method: string, url: string, form: array<string, string>, json?: array<string, mixed>, headers: array<string, string>}> */
    protected function requestsTo(string $url): array
    {
        return array_values(array_filter(
            $this->transport->requests(),
            static fn (array $request): bool => $request['url'] === $url,
        ));
    }

    /** @return array{method: string, url: string, form: array<string, string>, json?: array<string, mixed>, headers: array<string, string>} */
    protected function lastRequestTo(string $url): array
    {
        $requests = $this->requestsTo($url);
        self::assertNotSame([], $requests, 'No request was sent to ' . $url);

        return $requests[array_key_last($requests)];
    }
}
