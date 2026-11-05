-- Minecraft accounts linked to a user, and the Microsoft device-code sign-ins still waiting for the user.
-- A session row holds Microsoft's device code, which the server polls with. The row is deleted when the session ends.

CREATE TABLE minecraft_accounts (
    uuid CHAR(36) NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(16) NOT NULL,
    linked_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (uuid),
    KEY minecraft_accounts_user_idx (user_id),
    CONSTRAINT minecraft_accounts_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE minecraft_link_sessions (
    id CHAR(32) NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    device_code VARCHAR(512) NOT NULL,
    user_code VARCHAR(16) NOT NULL,
    verification_uri VARCHAR(255) NOT NULL,
    interval_seconds INT UNSIGNED NOT NULL,
    expires_at DATETIME(6) NOT NULL,
    last_polled_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY minecraft_link_sessions_user_idx (user_id),
    KEY minecraft_link_sessions_expires_idx (expires_at),
    CONSTRAINT minecraft_link_sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
