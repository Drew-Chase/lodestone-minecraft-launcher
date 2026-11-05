<?php

declare(strict_types=1);

namespace tests\Users;

use app\Users\UserRepository;
use tests\Support\DatabaseTestCase;

final class UserRepositoryTest extends DatabaseTestCase
{
    private UserRepository $users;

    protected function setUp(): void
    {
        parent::setUp();

        $this->users = new UserRepository();
    }

    public function testCreateThenFindByEmailReturnsTheRow(): void
    {
        $id = $this->users->create('find@example.test', 'hash-value', 'Finder');

        $row = $this->users->findByEmail('find@example.test');

        self::assertNotNull($row);
        self::assertSame($id, (int) $row['id']);
        self::assertSame('Finder', $row['display_name']);
        self::assertSame('hash-value', $row['password_hash']);
    }

    public function testCreateThenFindByIdReturnsTheRow(): void
    {
        $id = $this->users->create('byid@example.test', null, null);

        $row = $this->users->findById($id);

        self::assertNotNull($row);
        self::assertSame('byid@example.test', $row['email']);
        self::assertNull($row['password_hash']);
    }

    public function testFindByEmailForMissingAddressReturnsNull(): void
    {
        self::assertNull($this->users->findByEmail('nobody@example.test'));
    }

    public function testFindByIdForMissingUserReturnsNull(): void
    {
        self::assertNull($this->users->findById(999999));
    }

    public function testMarkVerifiedSetsTheTimestamp(): void
    {
        $id = $this->users->create('verify@example.test', null, null);
        self::assertNull($this->users->findById($id)['email_verified_at']);

        $this->users->markVerified($id);

        self::assertNotNull($this->users->findById($id)['email_verified_at']);
    }

    public function testMarkVerifiedTwiceKeepsTheFirstTimestamp(): void
    {
        $id = $this->users->create('twice@example.test', null, null);
        $this->users->markVerified($id);
        $first = $this->users->findById($id)['email_verified_at'];

        usleep(20000);
        $this->users->markVerified($id);

        self::assertSame($first, $this->users->findById($id)['email_verified_at']);
    }

    public function testSetPasswordHashStoresTheNewHash(): void
    {
        $id = $this->users->create('hash@example.test', 'old-hash', null);

        $this->users->setPasswordHash($id, 'new-hash');

        self::assertSame('new-hash', $this->users->findById($id)['password_hash']);
    }

    public function testToPublicHasExactlyTheDocumentedKeys(): void
    {
        $id = $this->users->create('shape@example.test', 'secret-hash', 'Shape');
        $public = $this->users->toPublic($this->users->findById($id));

        self::assertSame(
            ['id', 'email', 'email_verified', 'display_name', 'username', 'presence_hidden', 'created_at'],
            array_keys($public)
        );
        self::assertSame($id, $public['id']);
        self::assertSame('shape@example.test', $public['email']);
        self::assertFalse($public['email_verified']);
        self::assertSame('Shape', $public['display_name']);
        self::assertFalse($public['presence_hidden']);
    }

    public function testToPublicNeverExposesThePasswordHash(): void
    {
        $id = $this->users->create('nohash@example.test', 'secret-hash', null);

        $encoded = json_encode($this->users->toPublic($this->users->findById($id)), JSON_THROW_ON_ERROR);

        self::assertStringNotContainsString('secret-hash', $encoded);
    }

    public function testToPublicReportsVerifiedAfterMarkVerified(): void
    {
        $id = $this->users->create('flag@example.test', null, null);
        $this->users->markVerified($id);

        self::assertTrue($this->users->toPublic($this->users->findById($id))['email_verified']);
    }

    public function testToPublicCreatedAtIsIso8601Utc(): void
    {
        $id = $this->users->create('time@example.test', null, null);
        $public = $this->users->toPublic($this->users->findById($id));

        self::assertMatchesRegularExpression(
            '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00$/',
            $public['created_at']
        );
    }
}
