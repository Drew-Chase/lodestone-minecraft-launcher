-- The launch settings an instance overrides, as JSON. NULL, like the `{}` an instance is saved with, means
-- the instance inherits every setting from the global defaults. The JSON is read with serde defaults, so
-- a setting added later needs no migration.
ALTER TABLE instances ADD COLUMN settings TEXT;
