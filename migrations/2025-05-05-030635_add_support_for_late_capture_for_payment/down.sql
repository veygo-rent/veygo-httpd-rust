-- down.sql
ALTER TABLE payments
DROP
COLUMN IF EXISTS capture_before,
    DROP
COLUMN IF EXISTS amount_authorized
;