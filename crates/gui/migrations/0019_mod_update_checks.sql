-- The last full check of each instance's mods against their platforms: when it ran, and the newer release it found for each
-- jar, as a JSON object from file name to release. The Mods tab shows it at once on the next start, and checks again in the
-- background once it is old.
CREATE TABLE mod_update_checks (
    instance_id TEXT PRIMARY KEY NOT NULL,
    checked_at INTEGER NOT NULL,
    updates TEXT NOT NULL
);
