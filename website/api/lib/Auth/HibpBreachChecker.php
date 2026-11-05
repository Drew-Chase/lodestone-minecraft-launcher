<?php

declare(strict_types=1);

namespace app\Auth;

/**
 * Checks passwords against the Pwned Passwords range API (k-anonymity).
 * Only the first five hex characters of the SHA-1 leave the server.
 * When the service cannot be reached the password is accepted and the failure is logged.
 */
final class HibpBreachChecker implements BreachChecker
{
    public function __construct(private readonly string $endpoint = 'https://api.pwnedpasswords.com/range/')
    {
    }

    public function isBreached(string $password): bool
    {
        $hash = strtoupper(sha1($password));
        $prefix = substr($hash, 0, 5);
        $suffix = substr($hash, 5);

        $curl = curl_init($this->endpoint . $prefix);
        if ($curl === false) {
            error_log('Pwned Passwords check unavailable: curl_init failed');

            return false;
        }

        curl_setopt_array($curl, [
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_HTTPHEADER => ['Add-Padding: true'],
            CURLOPT_CONNECTTIMEOUT => 3,
            CURLOPT_TIMEOUT => 5,
            CURLOPT_FOLLOWLOCATION => false,
        ]);

        $body = curl_exec($curl);
        if ($body === false) {
            // The curl message can echo the URL, which holds the hash prefix, so only the code is logged.
            error_log('Pwned Passwords check unavailable: curl error ' . curl_errno($curl));

            return false;
        }

        $status = (int) curl_getinfo($curl, CURLINFO_RESPONSE_CODE);
        if ($status !== 200) {
            error_log('Pwned Passwords check unavailable: HTTP ' . $status);

            return false;
        }

        return self::matches(is_string($body) ? $body : '', $suffix);
    }

    /**
     * True when a line of the range response, `SUFFIX:COUNT`, has this suffix and a count above 0.
     * The suffix compares case-insensitively. Lines may end in CRLF, and padding lines have a count of 0.
     */
    public static function matches(string $body, string $suffix): bool
    {
        $wanted = strtoupper(trim($suffix));

        foreach (preg_split('/\r\n|\r|\n/', $body) ?: [] as $line) {
            $parts = explode(':', trim($line), 2);
            if (count($parts) !== 2) {
                continue;
            }

            if (strtoupper(trim($parts[0])) === $wanted && (int) trim($parts[1]) > 0) {
                return true;
            }
        }

        return false;
    }
}
