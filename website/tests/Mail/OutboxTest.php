<?php

declare(strict_types=1);

namespace tests\Mail;

use app\Mail\Mailer;
use app\Mail\MemoryMailer;
use app\Mail\Outbox;
use PHPUnit\Framework\TestCase;
use RuntimeException;

final class OutboxTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        Outbox::reset();
        (new MemoryMailer())->reset();
    }

    protected function tearDown(): void
    {
        Outbox::reset();
        (new MemoryMailer())->reset();

        parent::tearDown();
    }

    public function testSendDoesNotDeliverUntilFlush(): void
    {
        $memory = new MemoryMailer();
        $outbox = new Outbox();

        $outbox->send('a@example.test', 'Subject', 'text', 'html');

        self::assertSame([], $memory->sent());
    }

    public function testFlushDeliversEveryQueuedMessage(): void
    {
        $memory = new MemoryMailer();
        $outbox = new Outbox();
        $outbox->send('a@example.test', 'First', 'text 1', 'html 1');
        $outbox->send('b@example.test', 'Second', 'text 2', 'html 2');

        Outbox::flush(static fn (): Mailer => $memory);

        self::assertCount(2, $memory->sent());
        self::assertSame('First', $memory->sent()[0]['subject']);
        self::assertSame('b@example.test', $memory->sent()[1]['to']);
    }

    public function testSecondFlushSendsNothing(): void
    {
        $memory = new MemoryMailer();
        (new Outbox())->send('a@example.test', 'Once', 'text', 'html');

        Outbox::flush(static fn (): Mailer => $memory);
        Outbox::flush(static fn (): Mailer => $memory);

        self::assertCount(1, $memory->sent());
    }

    public function testFlushWithEmptyQueueDoesNotBuildTheTransport(): void
    {
        $built = false;

        Outbox::flush(static function () use (&$built): Mailer {
            $built = true;

            return new MemoryMailer();
        });

        self::assertFalse($built);
    }

    public function testFailingSendDoesNotMakeFlushThrowOrStopTheRest(): void
    {
        $outbox = new Outbox();
        $outbox->send('a@example.test', 'Broken', 'text', 'html');
        $outbox->send('b@example.test', 'Delivered', 'text', 'html');

        Outbox::flush(static fn (): Mailer => new class implements Mailer {
            public function send(string $to, string $subject, string $text, string $html): void
            {
                if ($to === 'a@example.test') {
                    throw new RuntimeException('smtp unavailable');
                }

                (new MemoryMailer())->send($to, $subject, $text, $html);
            }
        });

        $sent = (new MemoryMailer())->sent();
        self::assertCount(1, $sent);
        self::assertSame('b@example.test', $sent[0]['to']);
        self::assertSame('Delivered', $sent[0]['subject']);
    }

    public function testTransportThatCannotBeBuiltDoesNotMakeFlushThrow(): void
    {
        (new Outbox())->send('a@example.test', 'Subject', 'text', 'html');
        $attempted = false;

        Outbox::flush(static function () use (&$attempted): Mailer {
            $attempted = true;

            throw new RuntimeException('no transport');
        });

        self::assertTrue($attempted, 'flush() should try to build the transport when mail is queued.');
        self::assertSame([], (new MemoryMailer())->sent());
    }

    public function testQueueIsEmptiedEvenWhenTheTransportFails(): void
    {
        $memory = new MemoryMailer();
        (new Outbox())->send('a@example.test', 'Subject', 'text', 'html');

        Outbox::flush(static function (): Mailer {
            throw new RuntimeException('no transport');
        });
        Outbox::flush(static fn (): Mailer => $memory);

        self::assertSame([], $memory->sent());
    }
}
