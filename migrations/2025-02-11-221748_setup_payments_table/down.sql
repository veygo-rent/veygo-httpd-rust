-- This file should undo anything in `up.sql`
-- down.sql
DROP TABLE IF EXISTS payments;
DROP TYPE IF EXISTS payment_type_enum;