<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use app\Config\Config;
use app\Http\ErrorHandler;
use routes\AccountRoute;
use routes\AuthRoute;
use routes\BillingRoute;
use routes\DeviceRoute;
use routes\FriendsRoute;
use routes\MinecraftRoute;
use routes\OAuthRoute;
use routes\ReleaseRoute;
use Slim\Factory\AppFactory;

Config::load(__DIR__);

$app = AppFactory::create();
$app->setBasePath('/api');
$app->addRoutingMiddleware();

// Errors are JSON with the status they carry. Stack traces and messages stay out of the response.
$errors = $app->addErrorMiddleware(false, true, true);
// A first-class callable: Slim would otherwise try to construct ErrorHandler, whose constructor is private.
$errors->setDefaultErrorHandler(ErrorHandler::handle(...));

// ENDPOINTS
ReleaseRoute::configure($app);
AuthRoute::configure($app);
AccountRoute::configure($app);
OAuthRoute::configure($app);
BillingRoute::configure($app);
DeviceRoute::configure($app);
FriendsRoute::configure($app);
MinecraftRoute::configure($app);

return $app;
