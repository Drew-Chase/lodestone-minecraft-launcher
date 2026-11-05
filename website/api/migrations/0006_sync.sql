-- Sync: instances and their version history, content-addressed blobs, sharing and pending uploads.

CREATE TABLE instances (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    owner_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(64) NOT NULL,
    latest_version_id BIGINT UNSIGNED NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY instances_owner_idx (owner_id),
    CONSTRAINT instances_owner_fk FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE instance_versions (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    instance_id BIGINT UNSIGNED NOT NULL,
    pushed_by BIGINT UNSIGNED NOT NULL,
    device_id BIGINT UNSIGNED NULL,
    file_count INT UNSIGNED NOT NULL,
    total_bytes BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY instance_versions_instance_idx (instance_id, id),
    CONSTRAINT instance_versions_instance_fk FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE,
    CONSTRAINT instance_versions_pusher_fk FOREIGN KEY (pushed_by) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT instance_versions_device_fk FOREIGN KEY (device_id) REFERENCES devices (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE version_files (
    version_id BIGINT UNSIGNED NOT NULL,
    path VARCHAR(512) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
    sha256 CHAR(64) NOT NULL,
    size BIGINT UNSIGNED NOT NULL,
    mtime_ms BIGINT UNSIGNED NOT NULL,
    PRIMARY KEY (version_id, path),
    KEY version_files_sha_idx (sha256),
    CONSTRAINT version_files_version_fk FOREIGN KEY (version_id) REFERENCES instance_versions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE blobs (
    sha256 CHAR(64) NOT NULL,
    size BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (sha256)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE instance_shares (
    instance_id BIGINT UNSIGNED NOT NULL,
    friend_user_id BIGINT UNSIGNED NOT NULL,
    access VARCHAR(8) NOT NULL,
    granted_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (instance_id, friend_user_id),
    KEY instance_shares_friend_idx (friend_user_id),
    CONSTRAINT instance_shares_instance_fk FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE,
    CONSTRAINT instance_shares_friend_fk FOREIGN KEY (friend_user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE upload_sessions (
    id CHAR(32) NOT NULL,
    instance_id BIGINT UNSIGNED NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    device_id BIGINT UNSIGNED NULL,
    manifest JSON NOT NULL,
    file_count INT UNSIGNED NOT NULL,
    total_bytes BIGINT UNSIGNED NOT NULL,
    status VARCHAR(16) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at DATETIME(6) NOT NULL,
    PRIMARY KEY (id),
    KEY upload_sessions_instance_idx (instance_id),
    CONSTRAINT upload_sessions_instance_fk FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE,
    CONSTRAINT upload_sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT upload_sessions_device_fk FOREIGN KEY (device_id) REFERENCES devices (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
