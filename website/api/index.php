<?php

require_once __DIR__ . '/../vendor/autoload.php';

use routes\ReleaseRoute;
use Slim\Exception\HttpException;
use Slim\Factory\AppFactory;
use Slim\Psr7\Request;
use Slim\Psr7\Response;

$app = AppFactory::create();
$app->addRoutingMiddleware();
$app->setBasePath('/api');

// Errors are returned as JSON with the status they carry (404 stays 404).
// Stack traces are written to the server log only, never to the response.
$errors = $app->addErrorMiddleware(false, true, true);
$errors->setDefaultErrorHandler(
    function (Request $request, Throwable $exception) use ($app): Response
    {
        $status = $exception instanceof HttpException ? $exception->getCode() : 500;
        $response = $app->getResponseFactory()->createResponse($status);
        $response->getBody()->write(json_encode(['error' => $exception->getMessage()]));

        return $response->withHeader('Content-Type', 'application/json');
    }
);

// ENDPOINTS
ReleaseRoute::configure($app);

$app->run();
