ALTER TABLE payments
    ADD COLUMN amount_authorized DOUBLE PRECISION,
    ADD COLUMN capture_before TIMESTAMP WITH TIME ZONE
;