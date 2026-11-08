<?php

declare(strict_types=1);

namespace app\Http;

use app\Config\Config;
use Psr\Http\Message\ServerRequestInterface;

/**
 * The client's address, for rate limits and the audit log.
 *
 * The connection's address is used, and forwarded headers are read only when that connection comes from a proxy listed in
 * TRUSTED_PROXIES. Any client can send such a header, so it is believed only from a proxy the operator names. Behind a
 * trusted proxy, the client is the right-most X-Forwarded-For address that is not itself a trusted proxy.
 */
final class ClientIp
{
    private function __construct()
    {
    }

    public static function from(ServerRequestInterface $request): string
    {
        $address = $request->getServerParams()['REMOTE_ADDR'] ?? null;
        if (!is_string($address) || $address === '') {
            return '0.0.0.0';
        }

        $trusted = self::trustedProxies();
        if ($trusted === [] || !self::within($address, $trusted)) {
            return $address;
        }

        $hops = array_map('trim', explode(',', $request->getHeaderLine('X-Forwarded-For')));
        for ($index = count($hops) - 1; $index >= 0; $index--) {
            $hop = $hops[$index];
            // A malformed entry ends the chain: the proxy's own address is the best answer then.
            if (filter_var($hop, FILTER_VALIDATE_IP) === false) {
                break;
            }
            if (!self::within($hop, $trusted)) {
                return $hop;
            }
        }

        return $address;
    }

    /**
     * The addresses and ranges TRUSTED_PROXIES lists, such as `203.0.113.7` or `10.0.0.0/8`.
     *
     * @return list<string>
     */
    private static function trustedProxies(): array
    {
        $entries = array_map('trim', explode(',', Config::get('TRUSTED_PROXIES', '') ?? ''));

        return array_values(array_filter($entries, static fn (string $entry): bool => $entry !== ''));
    }

    /**
     * @param list<string> $ranges
     */
    private static function within(string $address, array $ranges): bool
    {
        foreach ($ranges as $range) {
            if (self::inRange($address, $range)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Whether `$address` lies in `$range`, an address or a CIDR range. An IPv4 address never matches an IPv6 range, and the
     * reverse.
     */
    private static function inRange(string $address, string $range): bool
    {
        [$network, $bits] = str_contains($range, '/') ? explode('/', $range, 2) : [$range, null];
        $packedAddress = @inet_pton($address);
        $packedNetwork = @inet_pton($network);
        if ($packedAddress === false || $packedNetwork === false || strlen($packedAddress) !== strlen($packedNetwork)) {
            return false;
        }

        $maxBits = strlen($packedNetwork) * 8;
        $prefix = $bits === null
            ? $maxBits
            : filter_var($bits, FILTER_VALIDATE_INT, ['options' => ['min_range' => 0, 'max_range' => $maxBits]]);
        if (!is_int($prefix)) {
            return false;
        }

        $wholeBytes = intdiv($prefix, 8);
        $leftoverBits = $prefix % 8;
        if (substr($packedAddress, 0, $wholeBytes) !== substr($packedNetwork, 0, $wholeBytes)) {
            return false;
        }
        if ($leftoverBits === 0) {
            return true;
        }

        $mask = (0xFF << (8 - $leftoverBits)) & 0xFF;

        return (ord($packedAddress[$wholeBytes]) & $mask) === (ord($packedNetwork[$wholeBytes]) & $mask);
    }
}
