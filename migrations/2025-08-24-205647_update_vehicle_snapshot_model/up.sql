ALTER TABLE agreements
DROP COLUMN IF EXISTS pickup_odometer,
DROP COLUMN IF EXISTS pickup_level,
DROP COLUMN IF EXISTS drop_off_odometer,
DROP COLUMN IF EXISTS drop_off_level;

ALTER TABLE vehicle_snapshots
    ADD COLUMN time TIMESTAMPTZ NOT NULL DEFAULT now(),
ADD COLUMN odometer INT NOT NULL,
ADD COLUMN level INT NOT NULL,
ADD COLUMN vehicle_id INT NOT NULL REFERENCES vehicles(id);
