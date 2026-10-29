-- The first-boot seed inserted demo rows with `seed-` ids. Real instances use
-- uuid ids, so this removes only the demo rows. Their folders are left on disk.
DELETE FROM instances WHERE id LIKE 'seed-%';
