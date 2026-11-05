<?php

declare(strict_types=1);

namespace app\Mail;

use app\Config\Config;
use Symfony\Component\Mailer\Mailer as SymfonyMailer;
use Symfony\Component\Mailer\Transport;

final class MailerFactory
{
    private const DEFAULT_FROM = 'noreply@lodestonelauncher.com';

    private function __construct()
    {
    }

    public static function fromConfig(): Mailer
    {
        $default = Config::isProduction() ? 'smtp' : 'file';
        $transport = Config::get('MAIL_TRANSPORT', $default) ?? $default;
        $from = Config::get('MAIL_FROM', self::DEFAULT_FROM) ?? self::DEFAULT_FROM;

        return match ($transport) {
            'memory' => new MemoryMailer(),
            'file' => new FileMailer(
                Config::get('MAIL_DIR') ?? Config::apiDir() . '/var/mail',
                $from,
            ),
            'smtp' => self::smtp($from),
            default => throw new \RuntimeException("Unknown MAIL_TRANSPORT: $transport"),
        };
    }

    private static function smtp(string $from): Mailer
    {
        $host = Config::need('SMTP_HOST');
        $port = (int) (Config::get('SMTP_PORT', '587') ?? '587');
        $scheme = $port === 465 ? 'smtps' : 'smtp';

        $auth = '';
        $user = Config::get('SMTP_USERNAME');
        if ($user !== null) {
            $password = Config::get('SMTP_PASSWORD');
            $auth = rawurlencode($user)
                . ($password !== null ? ':' . rawurlencode($password) : '')
                . '@';
        }

        $transport = Transport::fromDsn("$scheme://$auth$host:$port");

        return new SmtpMailer(new SymfonyMailer($transport), $from);
    }
}
