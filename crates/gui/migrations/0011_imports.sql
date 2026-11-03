-- The imports the Import Instance dialog lists as recent. A row is written when an import is queued, so an import that
-- failed or was cancelled still shows, and Reimport can start it again from its source.
CREATE TABLE IF NOT EXISTS imports (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    format      TEXT NOT NULL,
    origin      TEXT NOT NULL,
    source      TEXT NOT NULL,
    bytes       INTEGER NOT NULL,
    imported_at INTEGER NOT NULL
);
