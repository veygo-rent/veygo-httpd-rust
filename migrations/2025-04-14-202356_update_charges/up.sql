ALTER TABLE charges
    ALTER COLUMN agreement_id DROP NOT NULL,
    ADD COLUMN vehicle_id INTEGER NOT NULL REFERENCES vehicles (id);
