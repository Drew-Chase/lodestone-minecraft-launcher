-- Single sign-on. A row is one started sign-in. Only the SHA-256 of its state is stored, and each row is spent once.

CREATE TABLE oauth_states (
    state_hash CHAR(64) NOT NULL PRIMARY KEY,
    provider VARCHAR(16) NOT NULL,
    pkce_verifier VARCHAR(128) NULL,
    nonce VARCHAR(64) NULL,
    next_path VARCHAR(255) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at DATETIME(6) NOT NULL,
    used_at DATETIME(6) NULL,
    KEY oauth_states_expires_idx (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
