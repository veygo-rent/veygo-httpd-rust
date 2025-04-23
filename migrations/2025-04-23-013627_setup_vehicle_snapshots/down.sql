ALTER TABLE agreements
DROP
COLUMN IF EXISTS vehicle_snapshot_after,
    DROP
COLUMN IF EXISTS vehicle_snapshot_before,
    DROP
COLUMN IF EXISTS damage_ids;

DROP TABLE IF EXISTS vehicle_snapshots;