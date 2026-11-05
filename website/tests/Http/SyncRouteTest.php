<?php

declare(strict_types=1);

namespace tests\Http;

use app\Auth\JwtIssuer;
use app\Auth\Tokens;
use app\Storage\BlobKeys;
use app\Storage\BlobStores;
use Psr\Http\Message\ResponseInterface;
use Slim\App;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;
use tests\Storage\FakeBlobStore;
use tests\Support\DatabaseTestCase;
use tests\Sync\SyncFixtures;

/**
 * HTTP behaviour of /api/storage and /api/instances: instances, sharing, uploads, commits and version reads. The blob store
 * is a fake for the whole test, and the real one is put back afterwards.
 */
final class SyncRouteTest extends DatabaseTestCase
{
    use SyncFixtures;

    private const IP = '198.51.100.90';
    private const HOUR_LIMIT = 30;
    private const TWO_GIB = 2147483648;
    private const MTIME = 1700000000000;

    private static ?App $app = null;

    private FakeBlobStore $blobs;

    protected function setUp(): void
    {
        parent::setUp();

        $this->blobs = new FakeBlobStore();
        BlobStores::use($this->blobs);
    }

    protected function tearDown(): void
    {
        BlobStores::use(null);

        parent::tearDown();
    }

    public function test_storage_reports_the_owners_distinct_bytes_and_plan(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $this->insertVersion($instance, $owner, [
            ['path' => 'a.txt', 'sha256' => $this->digest('a'), 'size' => 10],
            ['path' => 'copy/a.txt', 'sha256' => $this->digest('a'), 'size' => 10],
            ['path' => 'b.txt', 'sha256' => $this->digest('b'), 'size' => 20],
        ]);

        $response = $this->call('GET', '/api/storage', as: $owner);

        $this->assertSame(200, $response->getStatusCode());
        self::assertSame([
            'used_bytes' => 30,
            'limit_bytes' => 100000000000,
            'plan' => 'solo',
            'name' => 'Solo Sync',
        ], $this->body($response));

        $free = $this->member('free@example.test', 'Free_1', true, null);
        self::assertSame(
            ['used_bytes' => 0, 'limit_bytes' => 0, 'plan' => 'free', 'name' => 'Free'],
            $this->body($this->call('GET', '/api/storage', as: $free)),
        );
    }

    public function test_creating_an_instance_needs_a_verified_email_and_a_plan(): void
    {
        $unverified = $this->member('new@example.test', 'New_1', false, 'solo');
        $free = $this->member('free@example.test', 'Free_1', true, null);

        $this->assertError($this->call('POST', '/api/instances', ['name' => 'Survival'], as: $unverified), 403, 'email_not_verified');
        $this->assertError($this->call('POST', '/api/instances', ['name' => 'Survival'], as: $free), 403, 'plan_required');
    }

    public function test_an_instance_name_is_one_to_sixty_four_characters_without_control_characters(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');

        $invalid = [
            ['name' => ''],
            ['name' => str_repeat('a', 65)],
            ['name' => "bad\nname"],
            ['name' => 5],
            ['other' => 'Survival'],
        ];
        foreach ($invalid as $body) {
            $this->assertError($this->call('POST', '/api/instances', $body, as: $owner), 400, 'invalid_name');
        }

        $this->assertSame(201, $this->call('POST', '/api/instances', ['name' => str_repeat('é', 64)], as: $owner)->getStatusCode());
        $this->assertSame(201, $this->call('POST', '/api/instances', ['name' => 'Survival'], as: $owner)->getStatusCode());
    }

    public function test_an_owner_creates_lists_and_reads_an_instance(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');

        $created = $this->call('POST', '/api/instances', ['name' => 'Survival'], as: $owner);
        $this->assertSame(201, $created->getStatusCode());

        $instance = $this->body($created)['instance'];
        self::assertIsInt($instance['id']);
        self::assertSame('Survival', $instance['name']);
        self::assertSame('owner', $instance['role']);
        self::assertSame(['username' => 'Ada_L'], $instance['owner']);
        self::assertNull($instance['latest_version']);
        self::assertIsString($instance['created_at']);

        $list = $this->body($this->call('GET', '/api/instances', as: $owner))['instances'];
        self::assertSame([$instance], $list);

        $shown = $this->call('GET', '/api/instances/' . $instance['id'], as: $owner);
        $this->assertSame(200, $shown->getStatusCode());
        self::assertSame(['instance' => $instance], $this->body($shown));
    }

    public function test_shared_instances_carry_the_granted_role_and_strangers_are_not_found(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $editor = $this->member('grace@example.test', 'Grace', true, null);
        $viewer = $this->member('linus@example.test', 'Linus', true, null);
        $stranger = $this->member('ken@example.test', 'Ken', true, null);

        $instance = $this->instance($owner);
        $this->share($instance, $editor, 'editor');
        $this->share($instance, $viewer, 'viewer');

        $editorList = $this->body($this->call('GET', '/api/instances', as: $editor))['instances'];
        self::assertCount(1, $editorList);
        self::assertSame('editor', $editorList[0]['role']);

        $viewerList = $this->body($this->call('GET', '/api/instances', as: $viewer))['instances'];
        self::assertSame('viewer', $viewerList[0]['role']);

        self::assertSame([], $this->body($this->call('GET', '/api/instances', as: $stranger))['instances']);

        $this->assertError($this->call('GET', "/api/instances/$instance", as: $stranger), 404, 'not_found');
        $this->assertSame('editor', $this->body($this->call('GET', "/api/instances/$instance", as: $editor))['instance']['role']);
    }

    public function test_only_the_owner_deletes_an_instance_and_its_storage_is_released(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $editor = $this->member('grace@example.test', 'Grace', true, null);
        $viewer = $this->member('linus@example.test', 'Linus', true, null);
        $stranger = $this->member('ken@example.test', 'Ken', true, null);

        $instance = $this->instance($owner);
        $this->share($instance, $editor, 'editor');
        $this->share($instance, $viewer, 'viewer');
        $this->insertVersion($instance, $owner, [['path' => 'a.txt', 'sha256' => $this->digest('a'), 'size' => 12]]);

        $this->assertError($this->call('DELETE', "/api/instances/$instance", as: $editor), 403, 'forbidden');
        $this->assertError($this->call('DELETE', "/api/instances/$instance", as: $viewer), 403, 'forbidden');
        $this->assertError($this->call('DELETE', "/api/instances/$instance", as: $stranger), 404, 'not_found');

        $this->assertSame(12, $this->body($this->call('GET', '/api/storage', as: $owner))['used_bytes']);

        $deleted = $this->call('DELETE', "/api/instances/$instance", as: $owner);
        $this->assertSame(204, $deleted->getStatusCode());

        $this->assertError($this->call('GET', "/api/instances/$instance", as: $owner), 404, 'not_found');
        self::assertSame([], $this->body($this->call('GET', '/api/instances', as: $owner))['instances']);
        self::assertSame(0, $this->body($this->call('GET', '/api/storage', as: $owner))['used_bytes']);
    }

    public function test_a_begin_presigns_one_put_for_each_blob_the_store_lacks(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $stored = $this->digest('stored');
        $fresh = $this->digest('fresh');
        // The owner's own version holds "stored", so the begin reuses it and presigns nothing for it.
        $this->insertVersion($instance, $owner, [['path' => 'old.txt', 'sha256' => $stored, 'size' => 5]]);

        $response = $this->beginUpload($instance, $owner, [
            $this->file('a.txt', 'stored', 5),
            $this->file('copy/a.txt', 'stored', 5),
            $this->file('c.txt', 'fresh', 7),
        ]);

        $this->assertSame(201, $response->getStatusCode());
        $body = $this->body($response);
        self::assertMatchesRegularExpression('/^[0-9a-f]{32}$/', $body['upload_id']);
        self::assertIsString($body['expires_at']);
        self::assertSame(1, $body['reused']);

        self::assertCount(1, $body['uploads']);
        $upload = $body['uploads'][0];
        self::assertSame($fresh, $upload['sha256']);
        self::assertSame(7, $upload['size']);
        self::assertStringContainsString('put', $upload['url']);
        self::assertSame(BlobKeys::base64Digest($fresh), $upload['headers']['x-amz-checksum-sha256']);

        $session = $this->pdo()->prepare('SELECT status, file_count, total_bytes FROM upload_sessions WHERE id = ?');
        $session->execute([$body['upload_id']]);
        self::assertSame(['status' => 'open', 'file_count' => 3, 'total_bytes' => 17], $session->fetch());
    }

    public function test_an_editor_may_begin_an_upload_a_viewer_may_not_and_a_stranger_is_not_found(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $editor = $this->member('grace@example.test', 'Grace', true, 'solo');
        $viewer = $this->member('linus@example.test', 'Linus', true, null);
        $stranger = $this->member('ken@example.test', 'Ken', true, 'solo');
        $instance = $this->instance($owner);
        $this->share($instance, $editor, 'editor');
        $this->share($instance, $viewer, 'viewer');

        $files = [$this->file('a.txt', 'a', 1)];
        $this->blob($this->digest('a'), 1);

        $this->assertSame(201, $this->beginUpload($instance, $editor, $files)->getStatusCode());
        $this->assertError($this->beginUpload($instance, $viewer, $files), 403, 'forbidden');
        $this->assertError($this->beginUpload($instance, $stranger, $files), 404, 'not_found');
    }

    public function test_an_upload_needs_a_verified_email_and_the_uploaders_plan(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $unverifiedEditor = $this->member('grace@example.test', 'Grace', false, 'solo');
        $freeEditor = $this->member('linus@example.test', 'Linus', true, null);
        $this->share($instance, $unverifiedEditor, 'editor');
        $this->share($instance, $freeEditor, 'editor');

        $files = [$this->file('a.txt', 'a', 1)];

        $this->assertError($this->beginUpload($instance, $unverifiedEditor, $files), 403, 'email_not_verified');
        $this->assertError($this->beginUpload($instance, $freeEditor, $files), 403, 'plan_required');
    }

    public function test_a_manifest_that_breaks_a_rule_is_invalid_manifest(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);

        $bad = [
            ['other' => 1],
            ['files' => []],
            ['files' => [$this->file('/abs.txt', 'a', 1)]],
            ['files' => [$this->file('a/../b.txt', 'a', 1)]],
            ['files' => [$this->file('a.txt', 'a', 1), $this->file('a.txt', 'b', 1)]],
            ['files' => [$this->rawFile('a.txt', strtoupper($this->digest('a')), 1)]],
            ['files' => [$this->rawFile('a.txt', $this->digest('a'), '5')]],
            ['files' => [$this->file('a.txt', 'a', 1, -1)]],
        ];

        foreach ($bad as $body) {
            $this->assertError($this->call('POST', "/api/instances/$instance/uploads", $body, as: $owner), 400, 'invalid_manifest');
        }

        $this->assertError(
            $this->call('POST', "/api/instances/$instance/uploads", raw: '{"files": [', as: $owner),
            400,
            'invalid_manifest',
        );
    }

    public function test_a_device_must_be_one_of_the_uploaders_active_devices(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $other = $this->member('grace@example.test', 'Grace', true, 'solo');
        $instance = $this->instance($owner);
        $this->pdo()->prepare("INSERT INTO devices (user_id, name, os, app_version) VALUES (?, 'Laptop', 'windows', '1.0.0')")
            ->execute([$other]);
        $otherDevice = (int) $this->pdo()->lastInsertId();

        $files = [$this->file('a.txt', 'a', 1)];
        $this->assertError(
            $this->beginUpload($instance, $owner, $files, device: $otherDevice),
            400,
            'invalid_device',
        );
    }

    public function test_an_upload_past_the_owners_allowance_is_refused(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);

        $files = [];
        for ($i = 0; $i < 46; $i++) {
            $files[] = ['path' => "world/$i.bin", 'sha256' => $this->digest("blob-$i"), 'size' => self::TWO_GIB];
        }
        $this->insertVersion($instance, $owner, $files);

        $this->assertError(
            $this->beginUpload($instance, $owner, [$this->file('new.bin', 'new', self::TWO_GIB)]),
            409,
            'quota_exceeded',
        );
    }

    public function test_upload_starts_are_limited_to_thirty_an_hour_per_user(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);

        for ($attempt = 0; $attempt < self::HOUR_LIMIT; $attempt++) {
            $this->assertSame(201, $this->beginUpload($instance, $owner, [$this->file('a.txt', 'a', 1)])->getStatusCode());
        }

        $limited = $this->beginUpload($instance, $owner, [$this->file('a.txt', 'a', 1)]);
        $this->assertError($limited, 429, 'rate_limited');
        self::assertNotSame('', $limited->getHeaderLine('Retry-After'));
    }

    public function test_a_commit_writes_the_version_once_the_objects_are_stored(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $upload = $this->body($this->beginUpload($instance, $owner, [
            $this->file('a.txt', 'a', 5),
            $this->file('dir/b.txt', 'b', 7),
        ]))['upload_id'];
        $this->blobs->put($this->digest('a'), 5);
        $this->blobs->put($this->digest('b'), 7);

        $response = $this->commit($instance, $upload, $owner);

        $this->assertSame(201, $response->getStatusCode());
        $version = $this->body($response)['version'];
        self::assertIsInt($version['id']);
        self::assertIsString($version['created_at']);
        self::assertSame(2, $version['file_count']);
        self::assertSame(12, $version['total_bytes']);
        self::assertSame('Ada_L', $version['pushed_by']);

        self::assertSame(2, (int) $this->pdo()->query('SELECT COUNT(*) FROM blobs')->fetchColumn());
        self::assertSame(2, (int) $this->pdo()->query('SELECT COUNT(*) FROM version_files WHERE version_id = ' . $version['id'])->fetchColumn());
        self::assertSame(
            $version['id'],
            (int) $this->pdo()->query("SELECT latest_version_id FROM instances WHERE id = $instance")->fetchColumn(),
        );

        $session = $this->pdo()->prepare('SELECT status FROM upload_sessions WHERE id = ?');
        $session->execute([$upload]);
        self::assertSame('committed', $session->fetchColumn());
    }

    public function test_an_incomplete_commit_lists_the_missing_blobs_and_leaves_the_session_open(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $a = $this->digest('a');
        $b = $this->digest('b');

        $upload = $this->body($this->beginUpload($instance, $owner, [
            $this->file('a.txt', 'a', 5),
            $this->file('b.txt', 'b', 7),
        ]))['upload_id'];

        $none = $this->commit($instance, $upload, $owner);
        $this->assertError($none, 409, 'upload_incomplete');
        self::assertSame([$a, $b], $this->body($none)['missing']);

        // A wrong size, a wrong checksum and an object last written by another session each count as missing.
        $this->blobs->put($a, 6);
        $this->blobs->put($b, 7);
        self::assertSame([$a], $this->body($this->commit($instance, $upload, $owner))['missing']);

        $this->blobs->put($a, 5, base64_encode(random_bytes(32)));
        self::assertSame([$a], $this->body($this->commit($instance, $upload, $owner))['missing']);

        $this->blobs->markStored($a, 5, null, 'another-session');
        self::assertSame([$a], $this->body($this->commit($instance, $upload, $owner))['missing']);

        $this->blobs->put($a, 5);
        $this->assertSame(201, $this->commit($instance, $upload, $owner)->getStatusCode());
    }

    public function test_a_second_commit_of_the_same_upload_is_upload_committed(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $upload = $this->body($this->beginUpload($instance, $owner, [$this->file('a.txt', 'a', 5)]))['upload_id'];
        $this->blobs->put($this->digest('a'), 5);

        $this->assertSame(201, $this->commit($instance, $upload, $owner)->getStatusCode());
        $this->assertError($this->commit($instance, $upload, $owner), 409, 'upload_committed');
    }

    public function test_an_expired_session_cannot_be_committed(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $this->blobs->markStored($this->digest('a'), 5);

        $upload = $this->body($this->beginUpload($instance, $owner, [$this->file('a.txt', 'a', 5)]))['upload_id'];
        $this->pdo()->prepare('UPDATE upload_sessions SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 MINUTE) WHERE id = ?')
            ->execute([$upload]);

        $this->assertError($this->commit($instance, $upload, $owner), 410, 'upload_expired');
    }

    public function test_only_the_uploader_commits_and_an_unknown_upload_is_not_found(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $editor = $this->member('grace@example.test', 'Grace', true, 'solo');
        $viewer = $this->member('linus@example.test', 'Linus', true, null);
        $instance = $this->instance($owner);
        $this->share($instance, $editor, 'editor');
        $this->share($instance, $viewer, 'viewer');
        $this->blobs->markStored($this->digest('a'), 5);

        $upload = $this->body($this->beginUpload($instance, $owner, [$this->file('a.txt', 'a', 5)]))['upload_id'];

        $this->assertError($this->commit($instance, $upload, $editor), 404, 'not_found');
        $this->assertError($this->commit($instance, $upload, $viewer), 403, 'forbidden');
        $this->assertError($this->commit($instance, str_repeat('0', 32), $owner), 404, 'not_found');
        $this->assertError($this->commit($instance, 'not-an-id', $owner), 404, 'not_found');
    }

    public function test_the_latest_and_one_version_reads_return_files_with_presigned_downloads(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $viewer = $this->member('linus@example.test', 'Linus', true, null);
        $stranger = $this->member('ken@example.test', 'Ken', true, null);
        $instance = $this->instance($owner);
        $this->share($instance, $viewer, 'viewer');

        $first = $this->push($instance, $owner, [$this->file('a.txt', 'a', 5)]);
        $second = $this->push($instance, $owner, [
            $this->file('a.txt', 'a', 5),
            $this->file('b.txt', 'b', 7),
        ]);

        $list = $this->call('GET', "/api/instances/$instance/versions", as: $owner);
        $this->assertSame(200, $list->getStatusCode());
        $versions = $this->body($list)['versions'];
        self::assertSame([$second['id'], $first['id']], [$versions[0]['id'], $versions[1]['id']]);
        self::assertSame(2, $versions[0]['file_count']);

        $latest = $this->call('GET', "/api/instances/$instance/versions/latest", as: $viewer);
        $this->assertSame(200, $latest->getStatusCode());
        $latestBody = $this->body($latest);
        self::assertSame($second['id'], $latestBody['version']['id']);
        self::assertCount(2, $latestBody['files']);
        self::assertSame(
            ['path' => 'a.txt', 'sha256' => $this->digest('a'), 'size' => 5, 'mtime_ms' => self::MTIME],
            array_diff_key($latestBody['files'][0], ['url' => true]),
        );
        self::assertStringContainsString('get', $latestBody['files'][0]['url']);

        $one = $this->call('GET', "/api/instances/$instance/versions/{$first['id']}", as: $owner);
        $this->assertSame(200, $one->getStatusCode());
        self::assertSame($first['id'], $this->body($one)['version']['id']);
        self::assertCount(1, $this->body($one)['files']);

        $this->assertError($this->call('GET', "/api/instances/$instance/versions/999999", as: $owner), 404, 'not_found');
        $this->assertError($this->call('GET', "/api/instances/$instance/versions/abc", as: $owner), 404, 'not_found');
        $this->assertError($this->call('GET', "/api/instances/$instance/versions", as: $stranger), 404, 'not_found');
        $this->assertError($this->call('GET', "/api/instances/$instance/versions/latest", as: $stranger), 404, 'not_found');
    }

    public function test_a_blob_another_owner_stored_is_uploaded_again_and_not_taken_on_trust(): void
    {
        $ada = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $grace = $this->member('grace@example.test', 'Grace', true, 'solo');
        $this->push($this->instance($grace, 'Grace world'), $grace, [$this->file('a.txt', 'a', 5)]);

        $instance = $this->instance($ada);
        $started = $this->body($this->beginUpload($instance, $ada, [$this->file('a.txt', 'a', 5)]));
        self::assertSame(0, $started['reused']);
        self::assertCount(1, $started['uploads']);

        // The store has the bytes, but Grace's session wrote them, so Ada's commit has not shown she had them.
        $incomplete = $this->commit($instance, $started['upload_id'], $ada);
        $this->assertError($incomplete, 409, 'upload_incomplete');
        self::assertSame([$this->digest('a')], $this->body($incomplete)['missing']);

        $this->blobs->put($this->digest('a'), 5);
        $this->assertSame(201, $this->commit($instance, $started['upload_id'], $ada)->getStatusCode());
    }

    public function test_a_commit_needs_the_uploaders_plan(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $upload = $this->body($this->beginUpload($instance, $owner, [$this->file('a.txt', 'a', 5)]))['upload_id'];
        $this->blobs->put($this->digest('a'), 5);

        $this->pdo()->prepare("UPDATE subscriptions SET status = 'canceled' WHERE user_id = ?")->execute([$owner]);

        $this->assertError($this->commit($instance, $upload, $owner), 403, 'plan_required');
    }

    public function test_a_manifest_that_disagrees_with_a_blob_the_owner_holds_is_refused(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);
        $this->push($instance, $owner, [$this->file('a.txt', 'a', 5)]);

        $this->assertError(
            $this->beginUpload($instance, $owner, [$this->file('b.txt', 'a', 6)]),
            400,
            'invalid_manifest',
        );
    }

    public function test_an_instance_with_no_version_has_no_latest(): void
    {
        $owner = $this->member('ada@example.test', 'Ada_L', true, 'solo');
        $instance = $this->instance($owner);

        $this->assertError($this->call('GET', "/api/instances/$instance/versions/latest", as: $owner), 404, 'not_found');
        self::assertSame([], $this->body($this->call('GET', "/api/instances/$instance/versions", as: $owner))['versions']);
    }

    /** Begins an upload and returns its id. */
    private function upload(int $instance, int $as, array $files): string
    {
        return (string) $this->body($this->beginUpload($instance, $as, $files))['upload_id'];
    }

    /**
     * Begins an upload, stores each blob it asked for the way the client's PUT would, commits, and returns the version.
     *
     * @param list<array<string, mixed>> $files
     * @return array<string, mixed>
     */
    private function push(int $instance, int $as, array $files): array
    {
        $started = $this->body($this->beginUpload($instance, $as, $files));
        foreach ($started['uploads'] as $upload) {
            $this->blobs->put($upload['sha256'], $upload['size']);
        }

        return $this->body($this->commit($instance, $started['upload_id'], $as))['version'];
    }

    /** @param list<array<string, mixed>> $files */
    private function beginUpload(int $instance, int $as, array $files, ?int $device = null): ResponseInterface
    {
        $body = ['files' => $files];
        if ($device !== null) {
            $body['device_id'] = $device;
        }

        return $this->call('POST', "/api/instances/$instance/uploads", $body, as: $as);
    }

    private function commit(int $instance, string $upload, int $as): ResponseInterface
    {
        return $this->call('POST', "/api/instances/$instance/uploads/$upload/commit", as: $as);
    }

    /**
     * A manifest entry for a seed. The digest is the SHA-256 of the seed, so two entries with one seed share a blob.
     *
     * @return array{path: string, sha256: string, size: int, mtime_ms: int}
     */
    private function file(string $path, string $seed, int $size, int $mtime = self::MTIME): array
    {
        return ['path' => $path, 'sha256' => $this->digest($seed), 'size' => $size, 'mtime_ms' => $mtime];
    }

    /** @return array<string, mixed> */
    private function rawFile(string $path, string $sha256, mixed $size): array
    {
        return ['path' => $path, 'sha256' => $sha256, 'size' => $size, 'mtime_ms' => self::MTIME];
    }

    /**
     * @param array<string, mixed>|null $json
     */
    private function call(
        string $method,
        string $path,
        ?array $json = null,
        ?string $raw = null,
        ?int $as = null,
    ): ResponseInterface {
        $request = (new ServerRequestFactory())->createServerRequest($method, $path, ['REMOTE_ADDR' => self::IP]);

        $payload = $raw ?? ($json === null ? null : json_encode($json, JSON_THROW_ON_ERROR));
        if ($payload !== null) {
            $request = $request
                ->withHeader('Content-Type', 'application/json')
                ->withBody((new StreamFactory())->createStream($payload));
        }

        if ($as !== null) {
            $token = JwtIssuer::fromConfig()->issue($as, Tokens::familyId())['token'];
            $request = $request->withHeader('Authorization', 'Bearer ' . $token);
        }

        return $this->app()->handle($request);
    }

    private function app(): App
    {
        if (self::$app === null) {
            self::$app = require dirname(__DIR__, 2) . '/api/bootstrap.php';
        }

        return self::$app;
    }

    /** @return array<string, mixed> */
    private function body(ResponseInterface $response): array
    {
        $decoded = json_decode((string) $response->getBody(), true, 512, JSON_THROW_ON_ERROR);
        $this->assertIsArray($decoded);

        return $decoded;
    }

    private function assertError(ResponseInterface $response, int $status, string $code): void
    {
        $this->assertSame($status, $response->getStatusCode());
        $this->assertSame($code, $this->body($response)['error'] ?? null);
    }
}
