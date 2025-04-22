ALTER TABLE charges
    ALTER COLUMN agreement_id DROP NOT NULL,
    ADD COLUMN vehicle_id INTEGER REFERENCES vehicles (id);
