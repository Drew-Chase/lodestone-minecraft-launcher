-- The release of an instance's modpack that it was installed with, as its platform names it. The update check compares the
-- newest release for the instance's Minecraft version with it. An instance made before this column has none until its
-- first check records the newest release as its baseline.
ALTER TABLE instances ADD COLUMN source_version TEXT;
