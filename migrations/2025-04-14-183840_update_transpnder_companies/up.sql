-- Your SQL goes here
ALTER TABLE transponder_companies
    ADD COLUMN timestamp_format VARCHAR NOT NULL DEFAULT '%Y-%m-%dT%H:%M:%SZ',
    ADD COLUMN utc_offset_hours  INTEGER;

-- Slash format, no zone in the string
UPDATE transponder_companies
SET timestamp_format = '%Y/%m/%d %H:%M:%S', -- 2025/04/13 17:44:24
    utc_offset_hours = -5                   -- CST (‑05:00 standard)
WHERE name = 'IPass';

-- Tesla ISO‑8601 with trailing Z (already UTC)
UPDATE transponder_companies
SET timestamp_format = '%Y-%m-%dT%H:%M:%SZ', -- 2025-04-08T00:35:21Z
    utc_offset_hours = NULL                  -- no extra shift
WHERE name = 'Tesla Charging';