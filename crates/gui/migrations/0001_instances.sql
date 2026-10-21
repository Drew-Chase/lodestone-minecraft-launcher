CREATE TABLE IF NOT EXISTS instances (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    mc_version       TEXT NOT NULL,
    loader           TEXT NOT NULL,
    loader_version   TEXT,
    playtime_seconds INTEGER NOT NULL DEFAULT 0,
    mod_count        INTEGER NOT NULL DEFAULT 0,
    last_played_at   INTEGER,
    installed_at     INTEGER NOT NULL,
    pinned           INTEGER NOT NULL DEFAULT 0,
    state            TEXT NOT NULL DEFAULT 'idle',
    biome            TEXT NOT NULL,
    art_seed         INTEGER NOT NULL,
    directory_name   TEXT NOT NULL,
    art_path         TEXT
);

CREATE INDEX IF NOT EXISTS instances_last_played ON instances (last_played_at);
