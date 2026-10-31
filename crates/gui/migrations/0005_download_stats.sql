-- The bytes the launcher downloaded on each local day, for the Downloads popover's totals.
CREATE TABLE IF NOT EXISTS download_stats (
    day   TEXT    PRIMARY KEY,
    bytes INTEGER NOT NULL
);
