-- The mods the Discover page added to an instance's `mods/` folder, so the Mods tab still knows them after a
-- restart. The key is the instance, the platform and the project. `file_name` is the jar in the folder.
CREATE TABLE instance_content (
    instance_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    project_id TEXT NOT NULL,
    version_id TEXT NOT NULL,
    name TEXT NOT NULL,
    file_name TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (instance_id, platform, project_id)
);
