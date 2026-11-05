<?php

declare(strict_types=1);

namespace routes;

use app\Auth\AuthException;
use app\Billing\BillingConfig;
use app\Billing\BillingSessions;
use app\Billing\Entitlements;
use app\Billing\Plans;
use app\Billing\SubscriptionStore;
use app\Billing\WebhookHandler;
use app\Http\BearerAuth;
use app\Http\Body;
use app\Http\Json;
use app\Users\UserRepository;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Slim\App;
use Stripe\StripeClient;

/**
 * Billing routes under /api/billing. The webhook takes no bearer token: its Stripe signature is the check.
 */
final class BillingRoute
{
    public static function configure(App $app): void
    {
        $app->get('/billing/plans', [self::class, 'plans']);
        $app->post('/billing/checkout', [self::class, 'checkout'])->add(new BearerAuth());
        $app->post('/billing/portal', [self::class, 'portal'])->add(new BearerAuth());
        $app->get('/billing/subscription', [self::class, 'subscription'])->add(new BearerAuth());
        $app->post('/billing/webhook', [self::class, 'webhook']);
    }

    /** GET /api/billing/plans */
    public static function plans(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $plans = [];
        foreach (Plans::paidIds() as $id) {
            $plan = Plans::get($id);

            $plans[] = [
                'id' => $id,
                'name' => $plan['name'],
                'storage_bytes' => $plan['storage_bytes'],
                'devices' => $plan['devices'],
                'shares' => $plan['shares'],
                'intervals' => $plan['intervals'],
            ];
        }

        return Json::respond($response, ['plans' => $plans]);
    }

    /** POST /api/billing/checkout */
    public static function checkout(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $prices = BillingConfig::prices();
        $stripe = BillingConfig::client();

        $body = Body::parse($request, true);

        $plan = $body['plan'] ?? null;
        $interval = $body['interval'] ?? null;
        if (!is_string($plan) || !is_string($interval)) {
            throw new AuthException('missing_field', 400);
        }

        if (!Plans::isPaid($plan) || !Plans::isInterval($interval)) {
            throw new AuthException('invalid_plan', 400);
        }

        $url = self::sessions($stripe)->checkout(
            (int) $request->getAttribute('user_id'),
            $plan,
            $interval,
            $prices,
        );

        return Json::respond($response, ['url' => $url]);
    }

    /** POST /api/billing/portal */
    public static function portal(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $stripe = BillingConfig::client();

        $url = self::sessions($stripe)->portal((int) $request->getAttribute('user_id'));

        return Json::respond($response, ['url' => $url]);
    }

    /** GET /api/billing/subscription */
    public static function subscription(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $subscription = Entitlements::for((int) $request->getAttribute('user_id'));

        return Json::respond($response, ['subscription' => $subscription]);
    }

    /** POST /api/billing/webhook */
    public static function webhook(ServerRequestInterface $request, ResponseInterface $response): ResponseInterface
    {
        $secret = BillingConfig::webhookSecret();
        $prices = BillingConfig::prices();

        $stream = $request->getBody();
        if ($stream->isSeekable()) {
            $stream->rewind();
        }

        $event = WebhookHandler::verify(
            (string) $stream,
            $request->getHeaderLine('Stripe-Signature'),
            $secret,
        );

        $handler = new WebhookHandler(new SubscriptionStore(), $prices, BillingConfig::client());
        $status = $handler->process($event);

        return Json::respond($response, ['status' => $status]);
    }

    private static function sessions(StripeClient $stripe): BillingSessions
    {
        return new BillingSessions(
            new SubscriptionStore(),
            new UserRepository(),
            $stripe,
            BillingConfig::appUrl(),
        );
    }
}
