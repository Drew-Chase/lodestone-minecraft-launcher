<?php

declare(strict_types=1);

namespace tests\Mail;

use app\Config\Config;
use app\Mail\FileMailer;
use app\Mail\MailerFactory;
use app\Mail\MemoryMailer;
use PHPUnit\Framework\TestCase;
use RuntimeException;

final class MailersTest extends TestCase
{
    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'lodestone-mail-' . bin2hex(random_bytes(6));
    }

    protected function tearDown(): void
    {
        Config::reset();

        if (is_dir($this->directory)) {
            foreach (scandir($this->directory) ?: [] as $name) {
                if ($name !== '.' && $name !== '..') {
                    unlink($this->directory . DIRECTORY_SEPARATOR . $name);
                }
            }
            rmdir($this->directory);
        }

        parent::tearDown();
    }

    public function testMemoryMailerKeepsSentMessages(): void
    {
        $mailer = new MemoryMailer();
        $mailer->reset();

        $mailer->send('a@example.test', 'Subject one', 'text one', '<p>one</p>');

        self::assertCount(1, $mailer->sent());
        self::assertSame(
            ['to' => 'a@example.test', 'subject' => 'Subject one', 'text' => 'text one', 'html' => '<p>one</p>'],
            $mailer->sent()[0]
        );
    }

    public function testMemoryMailerResetClearsMessages(): void
    {
        $mailer = new MemoryMailer();
        $mailer->send('a@example.test', 'Subject', 'text', 'html');

        $mailer->reset();

        self::assertSame([], $mailer->sent());
    }

    public function testFileMailerWritesOneTextFileWithSubjectAndText(): void
    {
        $mailer = new FileMailer($this->directory, 'noreply@example.test');

        $mailer->send('a@example.test', 'Verify please', 'Open the link below.', '<p>HTML-ONLY-MARKER</p>');

        $files = glob($this->directory . DIRECTORY_SEPARATOR . '*.txt') ?: [];
        self::assertCount(1, $files);

        $content = (string) file_get_contents($files[0]);
        self::assertStringContainsString('Subject: Verify please', $content);
        self::assertStringContainsString('To: a@example.test', $content);
        self::assertStringContainsString('Open the link below.', $content);
    }

    public function testFileMailerNeverWritesTheHtmlBody(): void
    {
        $mailer = new FileMailer($this->directory, 'noreply@example.test');

        $mailer->send('a@example.test', 'Subject', 'plain text', '<p>HTML-ONLY-MARKER</p>');

        $files = glob($this->directory . DIRECTORY_SEPARATOR . '*.txt') ?: [];
        self::assertStringNotContainsString('HTML-ONLY-MARKER', (string) file_get_contents($files[0]));
    }

    public function testFactoryWithMemoryTransportReturnsMemoryMailer(): void
    {
        Config::set('MAIL_TRANSPORT', 'memory');

        self::assertInstanceOf(MemoryMailer::class, MailerFactory::fromConfig());
    }

    public function testFactoryWithFileTransportReturnsFileMailer(): void
    {
        Config::set('MAIL_TRANSPORT', 'file');
        Config::set('MAIL_DIR', $this->directory);

        self::assertInstanceOf(FileMailer::class, MailerFactory::fromConfig());
    }

    public function testFactoryRejectsAnUnknownTransport(): void
    {
        Config::set('MAIL_TRANSPORT', 'carrier-pigeon');

        $this->expectException(RuntimeException::class);
        $this->expectExceptionMessage('Unknown MAIL_TRANSPORT: carrier-pigeon');

        MailerFactory::fromConfig();
    }
}
