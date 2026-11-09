-- The platform release a Discover-installed instance is on, by its id there. Installs and in-place updates write it, and
-- nothing else does. NULL means the release is unknown: an instance made before this column, or one whose update check
-- recorded only a baseline in `source_version`.
ALTER TABLE instances ADD COLUMN source_version_id TEXT;

-- The files the installed release shipped outside the user's own folders: its mods, config and scripts. `path` is relative to
-- the instance folder. `sha1` is the hash the release gives the file, or the hash read from disk when it gives none. An update
-- compares the instance's files with these rows, to tell what the pack placed from what the user added.
CREATE TABLE IF NOT EXISTS pack_files (
    instance_id TEXT NOT NULL,
    path TEXT NOT NULL,
    sha1 TEXT,
    PRIMARY KEY (instance_id, path)
);

-- The token of the last in-place pack update each instance committed. An update that stopped after its commit is finished
-- at the next launch, and one that stopped before it is rolled back, and this table is how the launch tells them apart.
CREATE TABLE IF NOT EXISTS pack_commits (
    instance_id TEXT PRIMARY KEY,
    token TEXT NOT NULL
);
