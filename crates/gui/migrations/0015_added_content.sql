-- The content the Discover page added to an instance: a pack in its resource or shader folder, a datapack in a world, or a
-- world imported as a save. The page shows it as added after a restart. `scope` is empty for an instance's own folders,
-- and the world's folder name for that world's datapacks. `file_name` is the file or folder written there.
CREATE TABLE added_content (
    instance_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    kind TEXT NOT NULL,
    platform TEXT NOT NULL,
    project_id TEXT NOT NULL,
    version_id TEXT NOT NULL,
    name TEXT NOT NULL,
    file_name TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (instance_id, scope, kind, platform, project_id)
);
