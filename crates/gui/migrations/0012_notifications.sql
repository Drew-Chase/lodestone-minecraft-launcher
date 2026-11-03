-- The notifications the bell lists: crashes, updates, and installs and exports that ended. A key names the event that
-- raised the notification, so an event recorded twice is kept once. The text is made when a row is shown, so it follows
-- the language, and the row keeps only what that text needs.
CREATE TABLE IF NOT EXISTS notifications (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    key         TEXT NOT NULL UNIQUE,
    kind        TEXT NOT NULL,
    subject     TEXT NOT NULL,
    detail      TEXT NOT NULL DEFAULT '',
    instance_id TEXT,
    target      TEXT,
    created_at  INTEGER NOT NULL,
    read_at     INTEGER
);
