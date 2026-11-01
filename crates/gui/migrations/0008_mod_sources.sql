-- Where each mod in an instance's `mods/` folder came from: the platform project and release it was installed as.
-- The key is the jar's file name, `.disabled` included, so a jar that was not installed from a platform has no row.
-- `sha1` is the hash the install was given, when the platform gave one.
CREATE TABLE mod_sources (
    instance_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    platform TEXT NOT NULL,
    project_id TEXT NOT NULL,
    version_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sha1 TEXT,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (instance_id, file_name)
);

-- Every row of `instance_content` was a jar the Discover page wrote into an instance's `mods/` folder.
INSERT OR IGNORE INTO mod_sources (instance_id, file_name, platform, project_id, version_id, name, sha1, added_at)
SELECT instance_id, file_name, platform, project_id, version_id, name, NULL, added_at FROM instance_content;

DROP TABLE instance_content;

-- The SHA-1 of a jar that has no row, cached by its file name, size and modification time, so each jar is hashed once.
CREATE TABLE jar_hashes (
    instance_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    sha1 TEXT NOT NULL,
    PRIMARY KEY (instance_id, file_name)
);
