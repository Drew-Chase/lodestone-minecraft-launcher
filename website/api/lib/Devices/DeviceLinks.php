<?php

declare(strict_types=1);

namespace app\Devices;

use app\Auth\AuthException;
use app\Auth\Tokens;
use app\Db\Database;
use PDO;
use PDOException;
use RuntimeException;

/**
 * Starts a launcher sign-in and approves it from the web. The launcher polls with its device code until the user
 * approves, and then redeems the approval through DeviceTokens.
 */
final class DeviceLinks
{
    public const TTL_SECONDS = 600;
    public const POLL_INTERVAL_SECONDS = 5;

    private const USER_CODE_ATTEMPTS = 5;
    private const SWEEP_ONE_IN = 100;
    private const SWEEP_AFTER_SECONDS = 86400;

    /**
     * @param array{name: string, os: string, app_version: string} $fields
     * @return array{device_code: string, user_code: string}
     */
    public function start(array $fields): array
    {
        // The device code is 32 random bytes in base64url. It is returned once, and only its hash is stored.
        $deviceCode = rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
        $hash = Tokens::hash($deviceCode);
        $pdo = Database::pdo();

        for ($attempt = 0; $attempt < self::USER_CODE_ATTEMPTS; $attempt++) {
            $userCode = ShortCode::random();

            try {
                $insert = $pdo->prepare(
                    "INSERT INTO device_links (user_code, device_code_hash, status, name, os, app_version, expires_at)
                     VALUES (?, ?, 'pending', ?, ?, ?, DATE_ADD(NOW(6), INTERVAL " . self::TTL_SECONDS . ' SECOND))'
                );
                $insert->execute([$userCode, $hash, $fields['name'], $fields['os'], $fields['app_version']]);
            } catch (PDOException $error) {
                // A duplicate user code is drawn again. Any other failure is a real error.
                if ($error->getCode() === '23000') {
                    continue;
                }

                throw $error;
            }

            $this->sweep();

            return ['device_code' => $deviceCode, 'user_code' => ShortCode::hyphenate($userCode)];
        }

        throw new RuntimeException('Could not allocate a device user code');
    }

    /**
     * Approves a pending sign-in for the user. Email verification is not needed.
     *
     * @throws AuthException invalid_code (404), code_expired (410), code_used (409) or device_limit_reached (409)
     */
    public function approve(int $userId, string $input): void
    {
        $userCode = ShortCode::normalize($input);
        if ($userCode === null) {
            throw new AuthException('invalid_code', 404);
        }

        Database::transaction(function (PDO $pdo) use ($userId, $userCode): void {
            $select = $pdo->prepare(
                'SELECT id, status, (expires_at <= NOW(6)) AS expired FROM device_links WHERE user_code = ? FOR UPDATE'
            );
            $select->execute([$userCode]);
            $row = $select->fetch(PDO::FETCH_ASSOC);

            if ($row === false) {
                throw new AuthException('invalid_code', 404);
            }

            if ((int) $row['expired'] === 1) {
                throw new AuthException('code_expired', 410);
            }

            if ($row['status'] !== 'pending') {
                throw new AuthException('code_used', 409);
            }

            (new Devices())->assertRoom($userId);

            $update = $pdo->prepare(
                "UPDATE device_links SET status = 'approved', user_id = ?, approved_at = NOW(6) WHERE id = ?"
            );
            $update->execute([$userId, (int) $row['id']]);
        });
    }

    /**
     * Reads the sign-in a user code belongs to, so the approver can check the device before approving. The checks and
     * their errors follow approve. Only the details to check are returned, never the device code or any hash.
     *
     * @return array{name: string, os: string, app_version: string, expires_at: string}
     * @throws AuthException invalid_code (404), code_expired (410) or code_used (409)
     */
    public function lookup(string $input): array
    {
        $userCode = ShortCode::normalize($input);
        if ($userCode === null) {
            throw new AuthException('invalid_code', 404);
        }

        $select = Database::pdo()->prepare(
            'SELECT name, os, app_version, expires_at, status, (expires_at <= NOW(6)) AS expired
             FROM device_links WHERE user_code = ?'
        );
        $select->execute([$userCode]);
        $row = $select->fetch(PDO::FETCH_ASSOC);

        if ($row === false) {
            throw new AuthException('invalid_code', 404);
        }

        if ((int) $row['expired'] === 1) {
            throw new AuthException('code_expired', 410);
        }

        if ($row['status'] !== 'pending') {
            throw new AuthException('code_used', 409);
        }

        return [
            'name' => (string) $row['name'],
            'os' => (string) $row['os'],
            'app_version' => (string) $row['app_version'],
            'expires_at' => Times::iso((string) $row['expires_at']),
        ];
    }

    /**
     * Deletes sign-ins that expired a day ago. A pending or consumed row is kept until then, so the codes
     * stay unique and a reused device code still reads as consumed.
     */
    private function sweep(): void
    {
        if (random_int(1, self::SWEEP_ONE_IN) !== 1) {
            return;
        }

        Database::pdo()->exec(
            'DELETE FROM device_links WHERE expires_at < DATE_SUB(NOW(6), INTERVAL ' . self::SWEEP_AFTER_SECONDS . ' SECOND)'
        );
    }
}
