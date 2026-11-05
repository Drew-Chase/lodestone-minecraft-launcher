-- utf8mb4_bin pads with spaces when it compares, so "a.txt" and "a.txt " shared one key and the second insert failed. The
-- no-pad collation keeps them apart, as the file systems the launcher syncs to do. A new migration, not an edit to 0006,
-- so databases that already ran 0006 change too.

ALTER TABLE version_files
    MODIFY path VARCHAR(512) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_bin NOT NULL;
