-- The page an instance was installed from, when it came from an external modpack source. Locally
-- created instances leave it empty.
ALTER TABLE instances ADD COLUMN source_url TEXT;
