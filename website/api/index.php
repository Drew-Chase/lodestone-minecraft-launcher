<?php

declare(strict_types=1);

use app\Mail\MailerFactory;
use app\Mail\Outbox;

$app = require __DIR__ . '/bootstrap.php';
$app->run();

// Account mail is sent after the response has gone out (see Outbox).
Outbox::flush(static fn () => MailerFactory::fromConfig());
