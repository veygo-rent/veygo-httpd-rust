-- This file should undo anything in `up.sql`
ALTER TABLE apartments
DROP COLUMN IF EXISTS uni_id;
ALTER TABLE apartments
    ADD COLUMN is_uni BOOLEAN NOT NULL DEFAULT false;