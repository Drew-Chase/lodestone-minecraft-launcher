<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\OAuth\Transport;

/**
 * Answers provider calls from canned responses keyed by URL, and records every request it receives.
 * An unknown URL throws, so a test that reaches for an unexpected address fails rather than using the network.
 */
final class FakeTransport implements Transport
{
    /** @var list<array{method: string, url: string, form: array<string, string>, json?: array<string, mixed>, headers: array<string, string>}> */
    private array $requests = [];

    /** @param array<string, array{status: int, body: string}> $responses Keyed by URL. */
    public function __construct(private array $responses = [])
    {
    }

    /**
     * @param array<string, mixed> $data
     * @return array{status: int, body: string}
     */
    public static function json(array $data, int $status = 200): array
    {
        return ['status' => $status, 'body' => json_encode($data, JSON_THROW_ON_ERROR)];
    }

    /** @param array{status: int, body: string} $response */
    public function respondWith(string $url, array $response): void
    {
        $this->responses[$url] = $response;
    }

    public function post(string $url, array $form, array $headers = []): array
    {
        $this->requests[] = ['method' => 'POST', 'url' => $url, 'form' => $form, 'headers' => $headers];

        return $this->respond($url);
    }

    /**
     * @param array<string, mixed> $json
     * @param array<string, string> $headers
     */
    public function postJson(string $url, array $json, array $headers = []): array
    {
        $this->requests[] = ['method' => 'POST', 'url' => $url, 'form' => [], 'json' => $json, 'headers' => $headers];

        return $this->respond($url);
    }

    public function get(string $url, array $headers = []): array
    {
        $this->requests[] = ['method' => 'GET', 'url' => $url, 'form' => [], 'headers' => $headers];

        return $this->respond($url);
    }

    /** @return list<array{method: string, url: string, form: array<string, string>, json?: array<string, mixed>, headers: array<string, string>}> */
    public function requests(): array
    {
        return $this->requests;
    }

    /** @return array{status: int, body: string} */
    private function respond(string $url): array
    {
        if (!array_key_exists($url, $this->responses)) {
            throw new \RuntimeException('No canned response for ' . $url);
        }

        return $this->responses[$url];
    }
}
