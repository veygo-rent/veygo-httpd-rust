-- This file should undo anything in `up.sql`

ALTER TABLE renters
    DROP COLUMN IF EXISTS apple_apns,
    DROP COLUMN IF EXISTS admin_apple_apns;
