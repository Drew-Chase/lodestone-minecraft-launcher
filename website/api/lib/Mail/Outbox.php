<?php

declare(strict_types=1);

namespace app\Mail;

/**
 * Holds account mail until the response has gone out. A request that sends mail then takes no longer
 * than one that does not, so response times cannot reveal which addresses have accounts.
 *
 * index.php calls flush() after the app has run. Under PHP-FPM, fastcgi_finish_request() ends the
 * response first, so the client does not wait for the SMTP conversation.
 */
final class Outbox implements Mailer
{
    /** @var list<array{to: string, subject: string, text: string, html: string}> */
    private static array $queue = [];

    public function send(string $to, string $subject, string $text, string $html): void
    {
        self::$queue[] = ['to' => $to, 'subject' => $subject, 'text' => $text, 'html' => $html];
    }

    /**
     * Sends everything queued, then empties the queue. The transport is built only when there is mail.
     * A failure is logged with the exception's class only, never the address or the body.
     *
     * @param \Closure(): Mailer $transport
     */
    public static function flush(\Closure $transport): void
    {
        if (self::$queue === []) {
            return;
        }

        $queued = self::$queue;
        self::$queue = [];

        if (function_exists('fastcgi_finish_request')) {
            fastcgi_finish_request();
        }

        try {
            $mailer = $transport();
        } catch (\Throwable $error) {
            error_log('Account email transport could not be built: ' . $error::class);

            return;
        }

        foreach ($queued as $message) {
            try {
                $mailer->send($message['to'], $message['subject'], $message['text'], $message['html']);
            } catch (\Throwable $error) {
                error_log('Account email could not be sent: ' . $error::class);
            }
        }
    }

    /** Drops anything queued without sending it. Tests only. */
    public static function reset(): void
    {
        self::$queue = [];
    }
}
