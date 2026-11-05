<?php

declare(strict_types=1);

namespace app\Devices;

use app\Auth\AuditLog;
use app\Auth\AuthException;
use app\Auth\JwtIssuer;
use app\Auth\RefreshTokens;
use app\Auth\Tokens;
use app\Db\Database;
use app\Users\UserRepository;
use PDO;

/**
 * Exchanges an approved device sign-in for a launcher session. The exchange spends the approval.
 */
final class DeviceTokens
{
    private const SLOW_DOWN_MICROSECONDS = 5000000;

    /**
     * @return array{access_token: string, expires_in: int, refresh_token: string, user: array<string, mixed>, device: array{id: int, name: string}}
     * @throws AuthException invalid_device_code, access_denied, expired_token, authorization_pending,
     *                       slow_down (400) or device_limit_reached (409)
     */
    public function redeem(string $deviceCode, string $ip): array
    {
        $hash = Tokens::hash($deviceCode);

        // A pending poll commits its last_polled_at. The pending error is thrown after the commit, so the
        // transaction returns the outcome rather than throwing it.
        $outcome = Database::transaction(function (PDO $pdo) use ($hash, $ip): array {
            $select = $pdo->prepare(
                'SELECT id, status, user_id, name, os, app_version, (expires_at <= NOW(6)) AS expired,
                        TIMESTAMPDIFF(MICROSECOND, last_polled_at, NOW(6)) AS since_poll
                 FROM device_links WHERE device_code_hash = ? FOR UPDATE'
            );
            $select->execute([$hash]);
            $row = $select->fetch(PDO::FETCH_ASSOC);

            if ($row === false) {
                throw new AuthException('invalid_device_code', 400);
            }

            $status = (string) $row['status'];

            if ($status === 'denied') {
                throw new AuthException('access_denied', 400);
            }

            if ($status === 'consumed') {
                throw new AuthException('expired_token', 400);
            }

            if ($status !== 'approved' && (int) $row['expired'] === 1) {
                throw new AuthException('expired_token', 400);
            }

            if ($status === 'pending') {
                if ($row['since_poll'] !== null && (int) $row['since_poll'] < self::SLOW_DOWN_MICROSECONDS) {
                    throw new AuthException('slow_down', 400, 5);
                }

                $touch = $pdo->prepare('UPDATE device_links SET last_polled_at = NOW(6) WHERE id = ?');
                $touch->execute([(int) $row['id']]);

                return ['pending' => true];
            }

            return ['pending' => false, 'session' => $this->grant($pdo, $row, $ip)];
        });

        if ($outcome['pending']) {
            throw new AuthException('authorization_pending', 400);
        }

        return $outcome['session'];
    }

    /**
     * Creates the device, its refresh family and the access token, then spends the approval. Runs inside the
     * redeem transaction, so a failure leaves the approval as it was.
     *
     * @param array<string, mixed> $row
     * @return array{access_token: string, expires_in: int, refresh_token: string, user: array<string, mixed>, device: array{id: int, name: string}}
     */
    private function grant(PDO $pdo, array $row, string $ip): array
    {
        $userId = (int) $row['user_id'];

        // The account row is locked, so two redeems for one user cannot both pass the device limit.
        $lock = $pdo->prepare('SELECT id FROM users WHERE id = ? FOR UPDATE');
        $lock->execute([$userId]);

        $users = new UserRepository();
        $user = $users->findById($userId);
        if ($user === null || $user['deleted_at'] !== null) {
            throw new AuthException('access_denied', 400);
        }

        $devices = new Devices();
        $devices->assertRoom($userId);

        $name = (string) $row['name'];
        $deviceId = $devices->add($userId, $name, (string) $row['os'], (string) $row['app_version']);

        $refresh = (new RefreshTokens())->issue($userId, 'launcher', $deviceId);
        $access = JwtIssuer::fromConfig()->issue($userId, $refresh['family_id']);

        $consume = $pdo->prepare(
            "UPDATE device_links SET status = 'consumed', consumed_at = NOW(6), device_id = ?
             WHERE id = ? AND status = 'approved'"
        );
        $consume->execute([$deviceId, (int) $row['id']]);

        AuditLog::recordOn($pdo, $userId, 'device_sign_in', $ip, null);

        return [
            'access_token' => $access['token'],
            'expires_in' => $access['expires_in'],
            'refresh_token' => $refresh['token'],
            'user' => $users->toPublic($user),
            'device' => ['id' => $deviceId, 'name' => $name],
        ];
    }
}
