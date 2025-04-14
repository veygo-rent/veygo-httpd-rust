-- This file should undo anything in `up.sql`
ALTER TABLE transponder_companies
DROP
COLUMN IF EXISTS timestamp_format,
    DROP
COLUMN IF EXISTS utc_offset_hours;