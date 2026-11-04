<?php

namespace routes;

use app\GitHubReleases;
use RuntimeException;
use Slim\App;
use Slim\Psr7\Request;
use Slim\Psr7\Response;

class ReleaseRoute
{
    static function configure(App $app): void
    {
        $app->get('/releases/latest', [self::class, 'latest']);
        $app->get('/stats', [self::class, 'stats']);
    }

    /** GET /api/releases/latest: the newest non-prerelease, with its downloads grouped by operating system. */
    static function latest(Request $request, Response $response): Response
    {
        try {
            $latest = GitHubReleases::latest();
        } catch (RuntimeException) {
            return self::json($response, ['error' => 'GitHub is unavailable'], 503);
        }

        return $latest === null
            ? self::json($response, ['error' => 'no releases found'], 404)
            : self::json($response, $latest);
    }

    /** GET /api/stats: stars, forks, open issues, license, total downloads and contributors. */
    static function stats(Request $request, Response $response): Response
    {
        try {
            return self::json($response, GitHubReleases::stats());
        } catch (RuntimeException) {
            return self::json($response, ['error' => 'GitHub is unavailable'], 503);
        }
    }

    /** @param array<string, mixed> $data */
    private static function json(Response $response, array $data, int $status = 200): Response
    {
        $response->getBody()->write(json_encode(
            $data,
            JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE
        ));

        return $response->withStatus($status)->withHeader('Content-Type', 'application/json');
    }
}
