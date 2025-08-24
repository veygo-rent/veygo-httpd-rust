ALTER TABLE agreements
    ADD COLUMN pickup_odometer INT,
    ADD COLUMN pickup_level INT,
    ADD COLUMN drop_off_odometer INT,
    ADD COLUMN drop_off_level INT;

ALTER TABLE vehicle_snapshots
DROP COLUMN IF EXISTS time,
    DROP COLUMN IF EXISTS odometer,
    DROP COLUMN IF EXISTS level,
    DROP COLUMN IF EXISTS vehicle_id;