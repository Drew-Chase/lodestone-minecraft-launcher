-- Icon, runtime and recents columns for instances made by the New Instance
-- flow. The `art_path` column from 0001 holds an uploaded banner.
ALTER TABLE instances ADD COLUMN icon_background TEXT;
ALTER TABLE instances ADD COLUMN icon_symbol TEXT;
ALTER TABLE instances ADD COLUMN icon_path TEXT;
ALTER TABLE instances ADD COLUMN java_component TEXT;
ALTER TABLE instances ADD COLUMN version_id TEXT;

-- The icons the icon editor offers under Recents, most recent last used.
CREATE TABLE IF NOT EXISTS icon_recents (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    background TEXT    NOT NULL,
    symbol     TEXT    NOT NULL,
    used_at    INTEGER NOT NULL
);
