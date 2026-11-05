<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\OAuth\GitHubProvider;
use app\Config\Config;
use PHPUnit\Framework\TestCase;

final class GitHubProviderTest extends TestCase
{
    private const CLIENT_ID = 'test-github-client-id';
    private const TOKEN_URL = 'https://github.com/login/oauth/access_token';
    private const USER_URL = 'https://api.github.com/user';
    private const EMAILS_URL = 'https://api.github.com/user/emails';
    private const REDIRECT_URI = 'https://app.test/api/auth/github/callback';

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('GITHUB_CLIENT_ID', self::CLIENT_ID);
        Config::set('GITHUB_CLIENT_SECRET', 'test-github-client-secret');
    }

    protected function tearDown(): void
    {
        Config::reset();

        parent::tearDown();
    }

    public function testTheAuthorizeUrlCarriesTheStateAndSendsNoPkceOrNonce(): void
    {
        $url = (new GitHubProvider())->authorizeUrl('the-state', 'unused-challenge', 'unused-nonce', self::REDIRECT_URI);

        self::assertStringStartsWith('https://github.com/login/oauth/authorize?', $url);
        parse_str((string) parse_url($url, PHP_URL_QUERY), $query);
        self::assertSame(self::CLIENT_ID, $query['client_id']);
        self::assertSame(self::REDIRECT_URI, $query['redirect_uri']);
        self::assertSame('read:user user:email', $query['scope']);
        self::assertSame('the-state', $query['state']);
        self::assertArrayNotHasKey('code_challenge', $query);
        self::assertArrayNotHasKey('nonce', $query);
    }

    public function testTheTokenRequestSendsTheCodeAndAsksForJson(): void
    {
        $transport = new FakeTransport([self::TOKEN_URL => FakeTransport::json(['access_token' => 'gh-access'])]);

        $tokens = (new GitHubProvider())->exchange('provider-code', 'unused-verifier', self::REDIRECT_URI, $transport);

        self::assertSame('gh-access', $tokens['access_token']);
        $requests = $transport->requests();
        self::assertSame('provider-code', $requests[0]['form']['code']);
        self::assertSame(self::CLIENT_ID, $requests[0]['form']['client_id']);
        self::assertSame('application/json', $requests[0]['headers']['Accept']);
    }

    public function testAnErrorAnswerWithoutAnAccessTokenIsRefused(): void
    {
        $transport = new FakeTransport([
            self::TOKEN_URL => FakeTransport::json(['error' => 'bad_verification_code']),
        ]);

        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('GitHub token response has no access token');

        (new GitHubProvider())->exchange('stale-code', 'unused-verifier', self::REDIRECT_URI, $transport);
    }

    public function testTheIdentityUsesTheUserIdAndThePrimaryVerifiedEmail(): void
    {
        $transport = $this->transport(
            ['id' => 4242, 'name' => 'Pilot'],
            [
                ['email' => 'old@example.test', 'primary' => false, 'verified' => true],
                ['email' => 'pilot@example.test', 'primary' => true, 'verified' => true],
            ],
        );

        $identity = (new GitHubProvider())->identity(['access_token' => 'gh-access'], 'unused-nonce', $transport);

        self::assertSame('github', $identity->provider);
        self::assertSame('4242', $identity->subject);
        self::assertSame('pilot@example.test', $identity->email);
        self::assertTrue($identity->emailVerified);
        self::assertSame('Pilot', $identity->name);
    }

    public function testTheUserAndEmailRequestsCarryTheAccessToken(): void
    {
        $transport = $this->transport(
            ['id' => 4242],
            [['email' => 'pilot@example.test', 'primary' => true, 'verified' => true]],
        );

        (new GitHubProvider())->identity(['access_token' => 'gh-access'], 'unused-nonce', $transport);

        $requests = $transport->requests();
        self::assertSame([self::USER_URL, self::EMAILS_URL], [$requests[0]['url'], $requests[1]['url']]);
        self::assertSame('Bearer gh-access', $requests[0]['headers']['Authorization']);
        self::assertSame('Bearer gh-access', $requests[1]['headers']['Authorization']);
    }

    public function testAPrimaryEmailThatIsNotVerifiedIsRefused(): void
    {
        $transport = $this->transport(
            ['id' => 4242],
            [['email' => 'pilot@example.test', 'primary' => true, 'verified' => false]],
        );

        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('GitHub account has no primary verified email');

        (new GitHubProvider())->identity(['access_token' => 'gh-access'], 'unused-nonce', $transport);
    }

    public function testAVerifiedEmailThatIsNotPrimaryIsRefused(): void
    {
        $transport = $this->transport(
            ['id' => 4242],
            [['email' => 'pilot@example.test', 'primary' => false, 'verified' => true]],
        );

        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('GitHub account has no primary verified email');

        (new GitHubProvider())->identity(['access_token' => 'gh-access'], 'unused-nonce', $transport);
    }

    public function testAMissingDisplayNameStillGivesAnIdentity(): void
    {
        $transport = $this->transport(
            ['id' => 4242],
            [['email' => 'pilot@example.test', 'primary' => true, 'verified' => true]],
        );

        $identity = (new GitHubProvider())->identity(['access_token' => 'gh-access'], 'unused-nonce', $transport);

        self::assertSame('pilot@example.test', $identity->email);
        self::assertNull($identity->name);
    }

    /**
     * @param array<string, mixed> $user
     * @param list<array<string, mixed>> $emails
     */
    private function transport(array $user, array $emails): FakeTransport
    {
        return new FakeTransport([
            self::USER_URL => FakeTransport::json($user),
            self::EMAILS_URL => FakeTransport::json($emails),
        ]);
    }
}
