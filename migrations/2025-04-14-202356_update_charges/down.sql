-- Revert: require `agreement_id` again
ALTER TABLE charges
    ALTER COLUMN agreement_id SET NOT NULL,
DROP
COLUMN IF EXISTS vehicle_id;