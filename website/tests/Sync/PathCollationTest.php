<?php

declare(strict_types=1);

namespace tests\Sync;

use tests\Support\DatabaseTestCase;

/** Paths are compared byte for byte, so a trailing space names a different file. */
final class PathCollationTest extends DatabaseTestCase
{
    use SyncFixtures;

    public function test_paths_that_differ_only_by_trailing_spaces_are_separate_files(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);

        $this->insertVersion($instance, $owner, [
            ['path' => 'a.txt', 'sha256' => $this->digest('a'), 'size' => 1],
            ['path' => 'a.txt ', 'sha256' => $this->digest('b'), 'size' => 2],
        ]);

        self::assertSame(2, (int) $this->pdo()->query('SELECT COUNT(*) FROM version_files')->fetchColumn());
    }
}
