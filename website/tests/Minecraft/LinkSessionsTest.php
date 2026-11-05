<?php

declare(strict_types=1);

namespace tests\Minecraft;

use app\Minecraft\DeviceCode;
use app\Minecraft\LinkSessions;
use tests\Support\DatabaseTestCase;

/**
 * The poll claim on its own: a session the user holds is claimed once per interval, and a missing or foreign
 * session is reported as absent, so the service can answer not_found rather than slow_down.
 */
final class LinkSessionsTest extends DatabaseTestCase
{
    use LinkFixtures;

    public function testAMissingOrForeignSessionIsReportedAbsentAndIsNotStamped(): void
    {
        $sessions = new LinkSessions();
        $owner = $this->createUser('steve@example.test');
        $stranger = $this->createUser('alex@example.test');
        $id = bin2hex(random_bytes(16));
        $sessions->insert(
            $id,
            $owner,
            new DeviceCode(self::DEVICE_CODE, self::USER_CODE, 'https://www.microsoft.com/link', 900, 5),
        );

        self::assertNull($sessions->claimPoll(bin2hex(random_bytes(16)), $owner));
        self::assertNull($sessions->claimPoll($id, $stranger));
        self::assertSame(
            0,
            $this->scalar('SELECT COUNT(*) FROM minecraft_link_sessions WHERE last_polled_at IS NOT NULL'),
        );
    }

    public function testTheOwnersFirstPollIsClaimedAndTheNextInsideTheIntervalIsNot(): void
    {
        $sessions = new LinkSessions();
        $owner = $this->createUser('steve@example.test');
        $id = bin2hex(random_bytes(16));
        $sessions->insert(
            $id,
            $owner,
            new DeviceCode(self::DEVICE_CODE, self::USER_CODE, 'https://www.microsoft.com/link', 900, 5),
        );

        self::assertTrue($sessions->claimPoll($id, $owner));
        self::assertFalse($sessions->claimPoll($id, $owner));
    }
}
