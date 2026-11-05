<?php

declare(strict_types=1);

namespace app\Mail;

/**
 * The text of each account email. Every value placed in the HTML body is escaped.
 *
 * Email links carry the token in the URL fragment, so browsers never send it to the server.
 */
final class AccountMail
{
    private function __construct()
    {
    }

    /** @return array{subject: string, text: string, html: string} */
    public static function verification(string $appUrl, string $token): array
    {
        $link = self::link($appUrl, '/verify-email', $token);

        return [
            'subject' => 'Verify your Lodestone email address',
            'text' => "Welcome to Lodestone.\n\n"
                . "Confirm your email address with this link:\n$link\n\n"
                . "The link is valid for 24 hours. If you did not create an account, you can ignore this email.\n",
            'html' => self::page(
                'Verify your email address',
                'Welcome to Lodestone. Confirm your email address with the button below. '
                . 'The link is valid for 24 hours. If you did not create an account, you can ignore this email.',
                'Verify email',
                $link,
            ),
        ];
    }

    /** @return array{subject: string, text: string, html: string} */
    public static function passwordReset(string $appUrl, string $token): array
    {
        $link = self::link($appUrl, '/reset-password', $token);

        return [
            'subject' => 'Reset your Lodestone password',
            'text' => "We received a request to reset your Lodestone password.\n\n"
                . "Choose a new password with this link:\n$link\n\n"
                . "The link is valid for one hour. If you did not ask for this, you can ignore this email.\n",
            'html' => self::page(
                'Reset your password',
                'We received a request to reset your Lodestone password. The link is valid for one hour. '
                . 'If you did not ask for this, you can ignore this email.',
                'Reset password',
                $link,
            ),
        ];
    }

    /**
     * Sent for an unverified account that someone registers again. Its link is a reset link, which sets a
     * password and verifies the address.
     *
     * @return array{subject: string, text: string, html: string}
     */
    public static function finishRegistration(string $appUrl, string $token): array
    {
        $link = self::link($appUrl, '/reset-password', $token);
        $intro = 'Someone asked to create a Lodestone account with this email address. '
            . 'An account for it already exists, so nothing has changed. '
            . 'The link below is valid for 24 hours. It lets the owner of this address choose a password '
            . 'and verify the address. If you did not ask for this, you can ignore this email.';

        return [
            'subject' => 'Finish creating your Lodestone account',
            'text' => "Someone asked to create a Lodestone account with this email address.\n\n"
                . "An account for it already exists, so nothing has changed.\n\n"
                . "The link below is valid for 24 hours. It lets the owner of this address choose a password "
                . "and verify the address:\n$link\n\n"
                . "If you did not ask for this, you can ignore this email.\n",
            'html' => self::page(
                'Finish creating your account',
                $intro,
                'Choose a password',
                $link,
            ),
        ];
    }

    /** @return array{subject: string, text: string, html: string} */
    public static function alreadyRegistered(string $appUrl): array
    {
        $signIn = self::escapeUrl(rtrim($appUrl, '/') . '/login');
        $reset = self::escapeUrl(rtrim($appUrl, '/') . '/forgot-password');

        return [
            'subject' => 'Your Lodestone account already exists',
            'text' => "Someone tried to create a Lodestone account with this email address, "
                . "but an account already exists for it, so nothing changed.\n\n"
                . "To sign in, go to $signIn\n"
                . "If you have forgotten your password, reset it at $reset\n\n"
                . "If this was not you, you can ignore this email.\n",
            'html' => self::htmlDocument(
                'Your account already exists',
                '<p>Someone tried to create a Lodestone account with this email address, but an account already exists for it, so nothing changed.</p>'
                . '<p>To sign in, go to <a href="' . $signIn . '">' . $signIn . '</a>.</p>'
                . '<p>If you have forgotten your password, reset it at <a href="' . $reset . '">' . $reset . '</a>.</p>'
                . '<p>If this was not you, you can ignore this email.</p>',
            ),
        ];
    }

    private static function link(string $appUrl, string $path, string $token): string
    {
        return rtrim($appUrl, '/') . $path . '#token=' . $token;
    }

    private static function escapeUrl(string $url): string
    {
        return htmlspecialchars($url, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8');
    }

    private static function page(string $heading, string $intro, string $buttonLabel, string $link): string
    {
        $escapedLink = self::escapeUrl($link);

        return self::htmlDocument(
            htmlspecialchars($heading, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8'),
            '<p>' . htmlspecialchars($intro, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8') . '</p>'
            . '<p><a href="' . $escapedLink . '">' . htmlspecialchars($buttonLabel, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8') . '</a></p>'
            . '<p>If the button does not work, copy this link into your browser:<br>' . $escapedLink . '</p>',
        );
    }

    private static function htmlDocument(string $title, string $body): string
    {
        return '<!DOCTYPE html><html><head><meta charset="utf-8"><title>' . $title . '</title></head>'
            . '<body style="font-family:sans-serif;">' . $body . '</body></html>';
    }
}
