-- The side a user chose for a mod on the server pack wizard's Review step, keyed by the mod's manifest id, so the same mod
-- keeps its choice in every instance and across restarts. `choice` is `include` or `exclude`. A mod with no row follows
-- the side its jar declares.
CREATE TABLE IF NOT EXISTS server_pack_sides (
    mod_id     TEXT PRIMARY KEY,
    choice     TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
