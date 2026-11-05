<?php

declare(strict_types=1);

namespace app\Mail;

/**
 * Writes each message to a text file, for development. The HTML body is never written.
 */
final class FileMailer implements Mailer
{
    public function __construct(
        private readonly string $directory,
        private readonly string $from,
    ) {
    }

    public function send(string $to, string $subject, string $text, string $html): void
    {
        if (!is_dir($this->directory) && !mkdir($this->directory, 0775, true) && !is_dir($this->directory)) {
            throw new \RuntimeException('Cannot create the mail directory');
        }

        $name = gmdate('Ymd-His') . '-' . bin2hex(random_bytes(4)) . '.txt';
        $path = $this->directory . DIRECTORY_SEPARATOR . $name;
        $content = "From: {$this->from}\nTo: $to\nSubject: $subject\n\n$text\n";

        if (file_put_contents($path, $content, LOCK_EX) === false) {
            throw new \RuntimeException('Cannot write the mail file');
        }
    }
}
