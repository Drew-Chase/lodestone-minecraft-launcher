<?php

declare(strict_types=1);

namespace app\Mail;

interface Mailer
{
    public function send(string $to, string $subject, string $text, string $html): void;
}
