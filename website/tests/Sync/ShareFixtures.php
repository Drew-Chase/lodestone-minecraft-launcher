<?php

declare(strict_types=1);

namespace tests\Sync;

use app\Users\UserRepository;
use PDO;

/** The rows the sharing tests need. Friendships, instances, plans and shares are written with direct SQL. */
final class ShareFixtures
{
    private function __construct()
    {
    }

    /** Creates an account with an optional username and a verified or unverified email, and returns its id. */
    public static function user(string $email, ?string $username, bool $verified = true): int
    {
        $users = new UserRepository();
        $id = $users->create($email, null, null);

        if ($username !== null) {
            $users->setUsername($id, $username);
        }

        if ($verified) {
            $users->markVerified($id);
        }

        return $id;
    }

    /** Writes a friendship row from requester to addressee and returns its id. */
    public static function friendship(PDO $pdo, int $requesterId, int $addresseeId, string $status = 'accepted'): int
    {
        $statement = $pdo->prepare('INSERT INTO friendships (requester_id, addressee_id, status) VALUES (?, ?, ?)');
        $statement->execute([$requesterId, $addresseeId, $status]);

        return (int) $pdo->lastInsertId();
    }

    /** Gives the user an active subscription to the plan. A free account has no row. */
    public static function plan(PDO $pdo, int $userId, string $plan): void
    {
        $statement = $pdo->prepare(
            "INSERT INTO subscriptions (user_id, plan, billing_interval, status) VALUES (?, ?, 'monthly', 'active')"
        );
        $statement->execute([$userId, $plan]);
    }

    /** Creates an instance owned by the user and returns its id. */
    public static function instance(PDO $pdo, int $ownerId, string $name = 'Survival'): int
    {
        $statement = $pdo->prepare('INSERT INTO instances (owner_id, name) VALUES (?, ?)');
        $statement->execute([$ownerId, $name]);

        return (int) $pdo->lastInsertId();
    }

    /** Writes a share row directly. */
    public static function share(PDO $pdo, int $instanceId, int $friendId, string $access = 'viewer'): void
    {
        $statement = $pdo->prepare('INSERT INTO instance_shares (instance_id, friend_user_id, access) VALUES (?, ?, ?)');
        $statement->execute([$instanceId, $friendId, $access]);
    }

    /** The number of share rows on the instance. */
    public static function shareCount(PDO $pdo, int $instanceId): int
    {
        $statement = $pdo->prepare('SELECT COUNT(*) FROM instance_shares WHERE instance_id = ?');
        $statement->execute([$instanceId]);

        return (int) $statement->fetchColumn();
    }
}
