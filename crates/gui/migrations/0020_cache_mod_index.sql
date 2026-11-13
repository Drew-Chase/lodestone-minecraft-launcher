-- The cache section. Every `cache_` table holds what the launcher can work out again, and Settings > Clear cache empties it.
--
-- What each mod jar declares, so the Mods tab does not open every jar again. A jar is known by its mods folder, its name
-- without `.disabled`, its size and its modification time in nanoseconds, so switching it on or off keeps its row and any
-- change to its bytes reads it again. `dependencies` is the declared list as JSON, and `icon` the jar's icon as a PNG,
-- scaled to the Mods tab's tile.
CREATE TABLE cache_mod_index (
    folder TEXT NOT NULL,
    stem TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified INTEGER NOT NULL,
    name TEXT,
    author TEXT,
    version TEXT,
    mod_id TEXT,
    dependencies TEXT NOT NULL,
    icon BLOB,
    PRIMARY KEY (folder, stem, size, modified)
);
