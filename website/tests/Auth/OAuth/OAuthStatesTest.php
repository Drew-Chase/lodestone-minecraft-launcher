<?php

declare(strict_types=1);

namespace tests\Auth\OAuth;

use app\Auth\OAuth\OAuthStates;
use tests\Support\DatabaseTestCase;

final class OAuthStatesTest extends DatabaseTestCase
{
    private OAuthStates $states;

    protected function setUp(): void
    {
        parent::setUp();

        $this->states = new OAuthStates();
    }

    public function testCreateThenConsumeReturnsTheStoredValuesOnce(): void
    {
        $created = $this->states->create('google', '/account/security');

        $consumed = $this->states->consume($created['state'], 'google');

        self::assertSame(
            ['pkce_verifier' => $created['verifier'], 'nonce' => $created['nonce'], 'next_path' => '/account/security'],
            $consumed,
        );
    }

    public function testASecondConsumeOfTheSameStateReturnsNull(): void
    {
        $created = $this->states->create('google', '/account');
        self::assertNotNull($this->states->consume($created['state'], 'google'));

        self::assertNull($this->states->consume($created['state'], 'google'));
    }

    public function testAnExpiredStateReturnsNull(): void
    {
        $created = $this->states->create('google', '/account');
        $this->pdo()->prepare(
            'UPDATE oauth_states SET expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND) WHERE state_hash = ?'
        )->execute([hash('sha256', $created['state'])]);

        self::assertNull($this->states->consume($created['state'], 'google'));
    }

    public function testOnlyTheHashOfTheStateIsStored(): void
    {
        $created = $this->states->create('google', '/account');

        $statement = $this->pdo()->query('SELECT state_hash FROM oauth_states');
        $stored = $statement->fetchColumn();

        self::assertNotSame($created['state'], $stored);
        self::assertSame(hash('sha256', $created['state']), $stored);
    }

    public function testAStateOnlyCompletesForTheProviderThatStartedIt(): void
    {
        $created = $this->states->create('google', '/account');

        self::assertNull($this->states->consume($created['state'], 'github'));
        self::assertNotNull($this->states->consume($created['state'], 'google'));
    }

    public function testTheCodeChallengeIsTheS256OfTheVerifier(): void
    {
        $created = $this->states->create('google', '/account');

        $expected = rtrim(strtr(base64_encode(hash('sha256', $created['verifier'], true)), '+/', '-_'), '=');

        self::assertSame($expected, $created['code_challenge']);
    }
}
