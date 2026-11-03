-- The servers the player saved, for the Servers page and the Join Server dialog. An address is kept once; a favorite is
-- listed first on the Servers page.
CREATE TABLE IF NOT EXISTS servers (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     TEXT NOT NULL,
    address  TEXT NOT NULL UNIQUE,
    favorite INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL
);
