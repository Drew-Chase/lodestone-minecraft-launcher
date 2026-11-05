<?php

declare(strict_types=1);

namespace app\Mail;

/**
 * Keeps sent messages in memory, for tests. The list is shared by every instance in the process.
 */
final class MemoryMailer implements Mailer
{
    /** @var list<array{to: string, subject: string, text: string, html: string}> */
    private static array $messages = [];

    public function send(string $to, string $subject, string $text, string $html): void
    {
        self::$messages[] = ['to' => $to, 'subject' => $subject, 'text' => $text, 'html' => $html];
    }

    /** @return list<array{to: string, subject: string, text: string, html: string}> */
    public static function sent(): array
    {
        return self::$messages;
    }

    public static function reset(): void
    {
        self::$messages = [];
    }
}
