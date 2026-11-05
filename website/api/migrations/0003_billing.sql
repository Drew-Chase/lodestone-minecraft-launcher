-- Billing: one subscription row per user, and the Stripe events already processed.

CREATE TABLE subscriptions (
    user_id BIGINT UNSIGNED NOT NULL,
    stripe_customer_id VARCHAR(191) NULL,
    stripe_subscription_id VARCHAR(191) NULL,
    plan VARCHAR(16) NULL,
    billing_interval VARCHAR(8) NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'none',
    trial_ends_at DATETIME(6) NULL,
    current_period_end DATETIME(6) NULL,
    cancel_at_period_end TINYINT(1) NOT NULL DEFAULT 0,
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (user_id),
    UNIQUE KEY subscriptions_customer_unique (stripe_customer_id),
    UNIQUE KEY subscriptions_stripe_subscription_unique (stripe_subscription_id),
    CONSTRAINT subscriptions_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE stripe_events (
    event_id VARCHAR(191) NOT NULL,
    type VARCHAR(64) NOT NULL,
    received_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (event_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
