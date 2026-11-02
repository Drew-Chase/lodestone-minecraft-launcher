-- Worlds the player pinned, by instance and save folder. The instances table has no foreign keys, so
-- `delete_instance` clears these rows itself.
CREATE TABLE IF NOT EXISTS world_pins (
    instance_id TEXT NOT NULL,
    folder      TEXT NOT NULL,
    PRIMARY KEY (instance_id, folder)
);
