<?php

declare(strict_types=1);

namespace app\Auth\OAuth;

/**
 * The network transport. Only Transports::current() reaches it by default. Tests replace it.
 */
final class CurlTransport implements Transport
{
    private const CONNECT_TIMEOUT_SECONDS = 5;
    private const TIMEOUT_SECONDS = 10;
    private const USER_AGENT = 'lodestone-website';

    public function post(string $url, array $form, array $headers = []): array
    {
        return $this->perform($url, $headers, static function ($handle) use ($form): void {
            curl_setopt($handle, CURLOPT_POST, true);
            curl_setopt($handle, CURLOPT_POSTFIELDS, http_build_query($form));
        });
    }

    public function postJson(string $url, array $json, array $headers = []): array
    {
        $body = json_encode($json, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);

        return $this->perform(
            $url,
            array_merge(['Content-Type' => 'application/json'], $headers),
            static function ($handle) use ($body): void {
                curl_setopt($handle, CURLOPT_POST, true);
                curl_setopt($handle, CURLOPT_POSTFIELDS, $body);
            },
        );
    }

    public function get(string $url, array $headers = []): array
    {
        return $this->perform($url, $headers, static function ($handle): void {
            curl_setopt($handle, CURLOPT_HTTPGET, true);
        });
    }

    /**
     * @param array<string, string> $headers
     * @param callable(\CurlHandle): void $configure
     * @return array{status: int, body: string}
     */
    private function perform(string $url, array $headers, callable $configure): array
    {
        $handle = curl_init($url);
        if ($handle === false) {
            throw new UpstreamFailure('HTTP request could not start');
        }

        $lines = [];
        foreach ($headers as $name => $value) {
            $lines[] = $name . ': ' . $value;
        }

        curl_setopt($handle, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($handle, CURLOPT_CONNECTTIMEOUT, self::CONNECT_TIMEOUT_SECONDS);
        curl_setopt($handle, CURLOPT_TIMEOUT, self::TIMEOUT_SECONDS);
        curl_setopt($handle, CURLOPT_USERAGENT, self::USER_AGENT);
        curl_setopt($handle, CURLOPT_HTTPHEADER, $lines);
        $configure($handle);

        $body = curl_exec($handle);
        if ($body === false) {
            // The error text names the failure only; it never includes the form or the headers.
            throw new UpstreamFailure('HTTP request failed: ' . curl_error($handle));
        }

        return [
            'status' => (int) curl_getinfo($handle, CURLINFO_RESPONSE_CODE),
            'body' => (string) $body,
        ];
    }
}
