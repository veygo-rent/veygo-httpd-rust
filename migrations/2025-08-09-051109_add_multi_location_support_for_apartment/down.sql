-- This file should undo anything in `up.sql`

-- 1) Remove agreement location link
ALTER TABLE agreements
    DROP COLUMN IF EXISTS location_id;

-- 2) Restore vehicles.apartment_id and remove location_id/remote_mgmt
ALTER TABLE vehicles
    ADD COLUMN IF NOT EXISTS apartment_id INT;

UPDATE vehicles v
SET apartment_id = l.apartment_id
FROM locations l
WHERE v.location_id = l.id
  AND v.apartment_id IS NULL;

ALTER TABLE vehicles
    ALTER COLUMN apartment_id SET NOT NULL;

ALTER TABLE vehicles
    ADD CONSTRAINT vehicles_apartment_id_fkey
    FOREIGN KEY (apartment_id) REFERENCES apartments(id) ON DELETE CASCADE;

ALTER TABLE vehicles
    DROP COLUMN IF EXISTS location_id,
    DROP COLUMN IF EXISTS remote_mgmt;

-- 3) Drop locations table
DROP TABLE IF EXISTS locations;

-- 4) Drop enum type last
DROP TYPE IF EXISTS remote_mgmt_enum;
