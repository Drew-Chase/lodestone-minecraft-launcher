-- The version number of the release each mod was installed as, as its platform names it, such as `0.6.0`. A record made
-- before this column has none, and reads as NULL until the mod is installed again. Snapshots write it, and fall back to the
-- version the jar declares when it is NULL.
ALTER TABLE mod_sources ADD COLUMN version_number TEXT;
