-- Remove the uniqueness constraint and the column
DROP INDEX IF EXISTS idx_charges_checksum;
ALTER TABLE charges
DROP
COLUMN IF EXISTS checksum,
DROP
COLUMN IF EXISTS transponder_company_id,
DROP
COLUMN IF EXISTS vehicle_identifier;