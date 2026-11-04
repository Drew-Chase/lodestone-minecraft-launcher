<?php

namespace app;

use RuntimeException;

/**
 * The launcher's GitHub releases and repository stats, cached for 30 minutes.
 *
 * Port of the Rust `ReleasesCache` that the old site used. PHP does not keep memory between
 * requests, so the cache is a JSON file in the system temp folder.
 */
final class GitHubReleases
{
    private const OWNER = 'Drew-Chase';
    private const REPO = 'lodestone-minecraft-launcher';
    private const API = 'https://api.github.com';
    private const TTL = 1800;   // seconds a fetched copy stays fresh
    private const RETRY = 300;  // seconds before retrying after GitHub fails and a stale copy is served

    /** @return array{release: array<string, mixed>, downloads: list<array<string, mixed>>}|null */
    public static function latest(): ?array
    {
        foreach (self::snapshot()['releases'] as $release) {
            if (!$release['prerelease']) {
                return ['release' => $release, 'downloads' => self::downloads($release['assets'])];
            }
        }

        return null;
    }

    /** @return array<string, mixed> */
    public static function stats(): array
    {
        $data = self::snapshot();

        $totalDownloads = 0;
        foreach ($data['releases'] as $release) {
            foreach ($release['assets'] as $asset) {
                $totalDownloads += $asset['download_count'];
            }
        }

        return [
            'stars' => $data['repo']['stars'],
            'total_downloads' => $totalDownloads,
            'forks' => $data['repo']['forks'],
            'open_issues' => $data['repo']['open_issues'],
            'license' => $data['repo']['license'],
            'license_name' => $data['repo']['license_name'],
            'contributors' => $data['contributors'],
        ];
    }

    /** @return array{fetched_at: int, releases: list<array<string, mixed>>, repo: array<string, mixed>, contributors: int} */
    private static function snapshot(): array
    {
        $file = self::cacheFile();
        $cached = self::readCache($file);
        if ($cached !== null && time() - $cached['fetched_at'] < self::TTL) {
            return $cached;
        }

        try {
            $fresh = self::fetch();
        } catch (RuntimeException $e) {
            if ($cached === null) {
                throw $e;
            }

            // GitHub is down or rate-limited: serve the last copy and try again in a few minutes.
            $cached['fetched_at'] = time() + self::RETRY - self::TTL;
            self::writeCache($file, $cached);

            return $cached;
        }

        self::writeCache($file, $fresh);

        return $fresh;
    }

    /** @return array{fetched_at: int, releases: list<array<string, mixed>>, repo: array<string, mixed>, contributors: int} */
    private static function fetch(): array
    {
        $releases = [];
        foreach (self::getJson(self::repoUrl('/releases?per_page=25')) as $release) {
            if (empty($release['draft'])) {
                $releases[] = self::release($release);
            }
        }

        $repo = self::getJson(self::repoUrl());

        return [
            'fetched_at' => time(),
            'releases' => $releases,
            'repo' => [
                'stars' => (int) ($repo['stargazers_count'] ?? 0),
                'forks' => (int) ($repo['forks_count'] ?? 0),
                'open_issues' => (int) ($repo['open_issues_count'] ?? 0),
                'license' => $repo['license']['spdx_id'] ?? null,
                'license_name' => $repo['license']['name'] ?? null,
            ],
            'contributors' => self::contributors(),
        ];
    }

    /**
     * @param array<string, mixed> $release a GitHub release object
     * @return array<string, mixed>
     */
    private static function release(array $release): array
    {
        $tag = (string) $release['tag_name'];

        $assets = [];
        foreach ($release['assets'] ?? [] as $asset) {
            $name = (string) $asset['name'];
            $assets[] = [
                'name' => $name,
                'download_url' => (string) $asset['browser_download_url'],
                'size' => (int) $asset['size'],
                'content_type' => (string) $asset['content_type'],
                'platform' => self::platform($name),
                'download_count' => (int) $asset['download_count'],
            ];
        }

        return [
            'version' => str_starts_with($tag, 'v') ? substr($tag, 1) : $tag,
            'tag' => $tag,
            'name' => (string) ($release['name'] ?? ''),
            'changelog' => (string) ($release['body'] ?? ''),
            'prerelease' => (bool) ($release['prerelease'] ?? false),
            'published_at' => $release['published_at'] ?? null,
            'html_url' => (string) $release['html_url'],
            'assets' => $assets,
        ];
    }

    /**
     * Groups a release's assets by operating system, in the order the site lists them.
     *
     * @param list<array<string, mixed>> $assets
     * @return list<array{platform: string, label: string, assets: list<array<string, mixed>>}>
     */
    private static function downloads(array $assets): array
    {
        $downloads = [];
        foreach (['windows' => 'Windows', 'macos' => 'macOS', 'linux' => 'Linux'] as $platform => $label) {
            $matched = array_values(array_filter($assets, fn(array $asset): bool => $asset['platform'] === $platform));
            if ($matched !== []) {
                $downloads[] = ['platform' => $platform, 'label' => $label, 'assets' => $matched];
            }
        }

        return $downloads;
    }

    /**
     * The operating system an asset installs on, matched by file name. Other files (SHA256SUMS.txt,
     * installer.sh, signatures) have no platform and are not offered as downloads.
     */
    private static function platform(string $name): ?string
    {
        $name = strtolower($name);

        if (preg_match('/(-setup\.exe|\.msi|-windows-x64\.zip)$/', $name)) {
            return 'windows';
        }
        if (preg_match('/(\.dmg|-macos-universal\.zip)$/', $name)) {
            return 'macos';
        }
        if (preg_match('/(\.deb|\.rpm|\.appimage|-linux-x86_64\.tar\.gz)$/', $name)) {
            return 'linux';
        }

        return null;
    }

    /**
     * The total contributor count: the last page number of a one-per-page listing, as the Rust
     * version did. Not critical, so a failure counts as zero rather than failing the refresh.
     */
    private static function contributors(): int
    {
        try {
            $response = self::get(self::repoUrl('/contributors?per_page=1&anon=true'));
        } catch (RuntimeException) {
            return 0;
        }

        if ($response['status'] !== 200) {
            return 0;
        }

        if (preg_match('/[?&]page=(\d+)>;\s*rel="last"/', $response['headers'], $match)) {
            return (int) $match[1];
        }

        $items = json_decode($response['body'], true);

        return is_array($items) ? count($items) : 0;
    }

    private static function repoUrl(string $path = ''): string
    {
        return self::API . '/repos/' . self::OWNER . '/' . self::REPO . $path;
    }

    /** @return array<mixed> */
    private static function getJson(string $url): array
    {
        $response = self::get($url);
        if ($response['status'] !== 200) {
            throw new RuntimeException("GitHub answered {$response['status']} for {$url}");
        }

        $data = json_decode($response['body'], true);
        if (!is_array($data)) {
            throw new RuntimeException("GitHub sent unreadable JSON for {$url}");
        }

        return $data;
    }

    /** @return array{status: int, headers: string, body: string} */
    private static function get(string $url): array
    {
        $ch = curl_init($url);
        if ($ch === false) {
            throw new RuntimeException('cURL could not start');
        }

        curl_setopt_array($ch, [
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_HEADER => true,
            CURLOPT_CONNECTTIMEOUT => 5,
            CURLOPT_TIMEOUT => 10,
            CURLOPT_HTTPHEADER => [
                'Accept: application/vnd.github+json',
                'X-GitHub-Api-Version: 2022-11-28',
                'User-Agent: lodestone-website', // GitHub rejects requests without a User-Agent
            ],
        ]);

        $raw = curl_exec($ch);
        if ($raw === false) {
            throw new RuntimeException('GitHub request failed: ' . curl_error($ch));
        }

        $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
        $headerSize = (int) curl_getinfo($ch, CURLINFO_HEADER_SIZE);

        return [
            'status' => $status,
            'headers' => substr($raw, 0, $headerSize),
            'body' => substr($raw, $headerSize),
        ];
    }

    private static function cacheFile(): string
    {
        return sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'lodestone-website' . DIRECTORY_SEPARATOR . 'github.json';
    }

    /** @return array{fetched_at: int, releases: list<array<string, mixed>>, repo: array<string, mixed>, contributors: int}|null */
    private static function readCache(string $file): ?array
    {
        $json = @file_get_contents($file);
        if ($json === false) {
            return null;
        }

        $data = json_decode($json, true);
        if (!is_array($data) || !isset($data['fetched_at'], $data['releases'], $data['repo'], $data['contributors'])) {
            return null;
        }

        return $data;
    }

    /** @param array<string, mixed> $data */
    private static function writeCache(string $file, array $data): void
    {
        $dir = dirname($file);
        if (!is_dir($dir) && !@mkdir($dir, 0775, true) && !is_dir($dir)) {
            return; // no writable temp folder: the next request fetches again
        }

        $json = json_encode($data, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);
        if ($json === false) {
            return;
        }

        // Write beside the cache, then rename, so a reader never sees a half-written file.
        $temp = $file . '.' . bin2hex(random_bytes(4));
        if (file_put_contents($temp, $json) !== false) {
            rename($temp, $file);
        }
    }
}
