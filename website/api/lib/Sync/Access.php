<?php

declare(strict_types=1);

namespace app\Sync;

use app\Db\Database;

/**
 * What a user may do with an instance, from the owner column and the share rows. Every sync and sharing check uses this.
 */
final class Access
{
    public const OWNER = 'owner';
    public const EDITOR = 'editor';
    public const VIEWER = 'viewer';

    private function __construct()
    {
    }

    /** The user's role on the instance, or null when the user has no access or the instance does not exist. */
    public static function roleFor(int $userId, int $instanceId): ?string
    {
        $statement = Database::pdo()->prepare('SELECT owner_id FROM instances WHERE id = ?');
        $statement->execute([$instanceId]);
        $ownerId = $statement->fetchColumn();
        if ($ownerId === false) {
            return null;
        }

        if ((int) $ownerId === $userId) {
            return self::OWNER;
        }

        $share = Database::pdo()->prepare(
            'SELECT access FROM instance_shares WHERE instance_id = ? AND friend_user_id = ?'
        );
        $share->execute([$instanceId, $userId]);
        $access = $share->fetchColumn();

        return is_string($access) && in_array($access, [self::EDITOR, self::VIEWER], true) ? $access : null;
    }
}
