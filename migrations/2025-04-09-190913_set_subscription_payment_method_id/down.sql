-- This file should undo anything in `up.sql`
ALTER TABLE renters
DROP
COLUMN IF EXISTS subscription_payment_method_id;