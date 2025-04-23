CREATE TABLE vehicle_snapshots
(
    id          SERIAL PRIMARY KEY,
    left_image  VARCHAR NOT NULL,
    right_image VARCHAR NOT NULL,
    front_image VARCHAR NOT NULL,
    back_image  VARCHAR NOT NULL
);

ALTER TABLE agreements
    ADD COLUMN damage_ids INT[] NOT NULL DEFAULT ARRAY[]::INT[],
    ADD COLUMN vehicle_snapshot_before INTEGER DEFAULT NULL,
    ADD COLUMN vehicle_snapshot_after  INTEGER DEFAULT NULL;