ALTER TABLE locations
    ADD COLUMN is_operational BOOLEAN NOT NULL DEFAULT true;