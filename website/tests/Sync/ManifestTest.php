<?php

declare(strict_types=1);

namespace tests\Sync;

use app\Auth\AuthException;
use app\Sync\Manifest;
use PHPUnit\Framework\TestCase;

/** The manifest rules of an upload. No database is needed. */
final class ManifestTest extends TestCase
{
    private const SHA_A = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    private const SHA_B = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

    public function test_a_valid_manifest_is_returned_in_stored_form(): void
    {
        $files = Manifest::validate([
            $this->file('mods/a.jar', self::SHA_A, 5, 1700000000000),
            $this->file('config/b.toml', self::SHA_B, 0, 0),
        ]);

        self::assertCount(2, $files);
        self::assertSame(
            ['path' => 'mods/a.jar', 'sha256' => self::SHA_A, 'size' => 5, 'mtime_ms' => 1700000000000],
            $files[0],
        );
        self::assertSame(0, $files[1]['size']);
    }

    public function test_a_manifest_decoded_from_json_objects_is_accepted(): void
    {
        $json = '{"files":[{"path":"a.txt","sha256":"' . self::SHA_A . '","size":3,"mtime_ms":10}]}';
        $body = json_decode($json);

        $files = Manifest::validate($body->files);

        self::assertSame('a.txt', $files[0]['path']);
    }

    public function test_paths_must_be_relative_with_forward_slashes(): void
    {
        $rejected = [
            '',
            '/abs.txt',
            'dir\\file.txt',
            'a//b',
            'a/',
            'a/./b',
            'a/../b',
            '..',
            '.',
            "bad\x01name",
            "tab\tname",
            "nul\x00name",
            'C:/Windows/file.txt',
            str_repeat('a', 513),
            "\xC3\x28",
        ];

        foreach ($rejected as $path) {
            $this->assertRejected([$this->file($path, self::SHA_A, 1, 1)], $path);
        }
    }

    public function test_paths_at_the_limits_and_dots_inside_names_are_accepted(): void
    {
        $accepted = [
            str_repeat('a', 512),
            'a..b',
            '.hidden',
            'dir/.../x',
            'résumé/日本語.txt',
        ];

        foreach ($accepted as $path) {
            $files = Manifest::validate([$this->file($path, self::SHA_A, 1, 1)]);
            self::assertSame($path, $files[0]['path']);
        }
    }

    public function test_paths_must_be_unique(): void
    {
        $this->assertRejected([
            $this->file('a.txt', self::SHA_A, 1, 1),
            $this->file('a.txt', self::SHA_B, 2, 2),
        ], 'duplicate path');
    }

    public function test_the_same_content_must_have_the_same_size(): void
    {
        $this->assertRejected([
            $this->file('a.txt', self::SHA_A, 1, 1),
            $this->file('b.txt', self::SHA_A, 2, 1),
        ], 'conflicting sizes');

        $files = Manifest::validate([
            $this->file('a.txt', self::SHA_A, 4, 1),
            $this->file('b.txt', self::SHA_A, 4, 1),
        ]);
        self::assertCount(2, $files);
    }

    public function test_sha256_must_be_64_lowercase_hex(): void
    {
        $bad = [
            strtoupper(self::SHA_A),
            substr(self::SHA_A, 1),
            self::SHA_A . 'a',
            str_repeat('g', 64),
            '',
        ];

        foreach ($bad as $sha) {
            $this->assertRejected([$this->file('a.txt', $sha, 1, 1)], 'sha256 ' . $sha);
        }
    }

    public function test_size_is_an_integer_from_zero_to_two_gibibytes(): void
    {
        $files = Manifest::validate([
            $this->file('zero.bin', self::SHA_A, 0, 1),
            $this->file('max.bin', self::SHA_B, 2147483648, 1),
        ]);
        self::assertSame(2147483648, $files[1]['size']);

        $bad = [-1, 2147483649, 1.5, '5', null];
        foreach ($bad as $size) {
            $this->assertRejected([$this->file('a.txt', self::SHA_A, $size, 1)], 'size ' . var_export($size, true));
        }
    }

    public function test_mtime_is_a_non_negative_integer(): void
    {
        Manifest::validate([$this->file('a.txt', self::SHA_A, 1, 0)]);

        $bad = [-1, 1.0, '1700000000000', null];
        foreach ($bad as $mtime) {
            $this->assertRejected([$this->file('a.txt', self::SHA_A, 1, $mtime)], 'mtime ' . var_export($mtime, true));
        }
    }

    public function test_every_file_field_is_required(): void
    {
        $entry = $this->file('a.txt', self::SHA_A, 1, 1);

        foreach (['path', 'sha256', 'size', 'mtime_ms'] as $field) {
            $missing = $entry;
            unset($missing[$field]);
            $this->assertRejected([$missing], 'missing ' . $field);
        }
    }

    public function test_the_file_count_is_from_one_to_fifty_thousand(): void
    {
        $this->assertRejected([], 'empty list');
        $this->assertRejected('not a list', 'string');

        $fifty = [];
        for ($i = 0; $i < Manifest::MAX_FILES; $i++) {
            $fifty[] = $this->file('f' . $i, hash('sha256', (string) $i), 1, 1);
        }
        self::assertCount(Manifest::MAX_FILES, Manifest::validate($fifty));

        $fifty[] = $this->file('one-more', self::SHA_A, 1, 1);
        $this->assertRejected($fifty, 'fifty thousand and one');
    }

    public function test_blobs_are_distinct_and_the_total_counts_every_file(): void
    {
        $files = Manifest::validate([
            $this->file('a.txt', self::SHA_A, 10, 1),
            $this->file('copy/a.txt', self::SHA_A, 10, 1),
            $this->file('b.txt', self::SHA_B, 7, 1),
        ]);

        self::assertSame([self::SHA_A => 10, self::SHA_B => 7], Manifest::blobs($files));
        self::assertSame(27, Manifest::totalBytes($files));
    }

    /** @return array{path: mixed, sha256: mixed, size: mixed, mtime_ms: mixed} */
    private function file(mixed $path, mixed $sha256, mixed $size, mixed $mtime): array
    {
        return ['path' => $path, 'sha256' => $sha256, 'size' => $size, 'mtime_ms' => $mtime];
    }

    /** @param mixed $files */
    private function assertRejected(mixed $files, string $why): void
    {
        try {
            Manifest::validate($files);
        } catch (AuthException $error) {
            self::assertSame('invalid_manifest', $error->errorCode, $why);
            self::assertSame(400, $error->httpStatus, $why);

            return;
        }

        self::fail("The manifest was accepted: $why");
    }
}
