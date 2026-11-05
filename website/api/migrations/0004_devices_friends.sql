-- Usernames, launcher devices, device sign-in links, friend codes, friendships and friend invites.

ALTER TABLE users
    ADD COLUMN username VARCHAR(16) CHARACTER SET ascii COLLATE ascii_general_ci NULL,
    ADD UNIQUE KEY users_username_unique (username);

-- A refresh family started by a launcher carries the device it belongs to. Rotation copies the id onto each new token.
ALTER TABLE refresh_tokens
    ADD COLUMN device_id BIGINT UNSIGNED NULL,
    ADD KEY refresh_tokens_device_idx (device_id);

-- A launcher installation signed in with a device code. Revoked devices stay as rows for the audit trail.
CREATE TABLE devices (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    user_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(64) NOT NULL,
    os VARCHAR(32) NOT NULL,
    app_version VARCHAR(32) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    last_seen_at DATETIME(6) NULL,
    revoked_at DATETIME(6) NULL,
    PRIMARY KEY (id),
    KEY devices_user_idx (user_id),
    CONSTRAINT devices_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- One row per device sign-in. Only the SHA-256 of the device code is stored. The user code is typed on the web page.
CREATE TABLE device_links (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    user_code CHAR(8) NOT NULL,
    device_code_hash CHAR(64) NOT NULL,
    status VARCHAR(16) NOT NULL,
    user_id BIGINT UNSIGNED NULL,
    name VARCHAR(64) NOT NULL,
    os VARCHAR(32) NOT NULL,
    app_version VARCHAR(32) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at DATETIME(6) NOT NULL,
    last_polled_at DATETIME(6) NULL,
    approved_at DATETIME(6) NULL,
    consumed_at DATETIME(6) NULL,
    device_id BIGINT UNSIGNED NULL,
    PRIMARY KEY (id),
    UNIQUE KEY device_links_user_code_unique (user_code),
    UNIQUE KEY device_links_device_code_unique (device_code_hash),
    CONSTRAINT device_links_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- The code a user gives out so that others can send them a friend request. One per account.
CREATE TABLE friend_codes (
    user_id BIGINT UNSIGNED NOT NULL,
    code CHAR(8) NOT NULL,
    PRIMARY KEY (user_id),
    UNIQUE KEY friend_codes_code_unique (code),
    CONSTRAINT friend_codes_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- A request is pending until the addressee accepts it. Accepted rows are friendships.
CREATE TABLE friendships (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    requester_id BIGINT UNSIGNED NOT NULL,
    addressee_id BIGINT UNSIGNED NOT NULL,
    status VARCHAR(16) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    accepted_at DATETIME(6) NULL,
    PRIMARY KEY (id),
    UNIQUE KEY friendships_pair_unique (requester_id, addressee_id),
    KEY friendships_addressee_idx (addressee_id, status),
    KEY friendships_requester_idx (requester_id, status),
    CONSTRAINT friendships_requester_fk FOREIGN KEY (requester_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT friendships_addressee_fk FOREIGN KEY (addressee_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- A single-use invite link. Only the SHA-256 of its token is stored.
CREATE TABLE friend_invites (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    inviter_id BIGINT UNSIGNED NOT NULL,
    token_hash CHAR(64) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at DATETIME(6) NOT NULL,
    used_by BIGINT UNSIGNED NULL,
    used_at DATETIME(6) NULL,
    PRIMARY KEY (id),
    UNIQUE KEY friend_invites_token_unique (token_hash),
    KEY friend_invites_inviter_idx (inviter_id),
    CONSTRAINT friend_invites_inviter_fk FOREIGN KEY (inviter_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT friend_invites_used_by_fk FOREIGN KEY (used_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
