<?php

declare(strict_types=1);

namespace app\Mail;

/**
 * Sends through a Symfony mailer. The Symfony classes are named in full so they do not clash with this namespace's Mailer.
 */
final class SmtpMailer implements Mailer
{
    public function __construct(
        private readonly \Symfony\Component\Mailer\MailerInterface $mailer,
        private readonly string $from,
    ) {
    }

    public function send(string $to, string $subject, string $text, string $html): void
    {
        $email = (new \Symfony\Component\Mime\Email())
            ->from(new \Symfony\Component\Mime\Address($this->from, 'Lodestone'))
            ->to($to)
            ->subject($subject)
            ->text($text)
            ->html($html);

        $this->mailer->send($email);
    }
}
