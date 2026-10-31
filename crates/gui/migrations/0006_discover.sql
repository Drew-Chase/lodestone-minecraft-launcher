-- Where an instance came from, when it came from a content platform: the platform's key and the
-- project's id there. Locally created instances leave both empty.
ALTER TABLE instances ADD COLUMN source_platform TEXT;
ALTER TABLE instances ADD COLUMN source_project_id TEXT;

-- The content the Discover page downloaded into the shared folders, so any instance can import it.
-- `kind` names the shared folder (`resourcepacks`, ...), and `path` is relative to that folder.
CREATE TABLE shared_content (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    platform TEXT NOT NULL,
    project_id TEXT NOT NULL,
    version_id TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER,
    added_at INTEGER NOT NULL,
    UNIQUE (kind, platform, project_id, version_id)
);
