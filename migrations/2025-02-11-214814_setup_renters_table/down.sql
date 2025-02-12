-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS renters;
DROP TYPE IF EXISTS gender_enum;
DROP TYPE IF EXISTS plan_tier_enum;
DROP TYPE IF EXISTS employee_tier_enum;