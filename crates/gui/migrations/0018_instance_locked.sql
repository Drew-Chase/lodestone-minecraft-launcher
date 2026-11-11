-- Whether the instance is locked to the modpack it was installed from. A locked instance keeps the mods, the loader and the
-- Minecraft version its pack set, and its modpack release, until the user unlocks it. Only a Discover modpack install locks
-- one, so every instance made before this column is unlocked.
ALTER TABLE instances ADD COLUMN locked INTEGER NOT NULL DEFAULT 0;
