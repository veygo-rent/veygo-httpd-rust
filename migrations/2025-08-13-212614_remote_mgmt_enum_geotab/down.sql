-- This file should undo anything in `up.sql`

-- Revert: remove 'geotab' from remote_mgmt_enum by recreating the type without it
-- NOTE: This assumes only `vehicles.remote_mgmt` uses the enum. Adjust other tables/columns similarly if needed.

DO $$
BEGIN
  -- 1) Create a temporary enum without 'geotab'
  IF NOT EXISTS (
    SELECT 1 FROM pg_type WHERE typname = 'remote_mgmt_enum_old'
  ) THEN
    CREATE TYPE remote_mgmt_enum_old AS ENUM ('revers', 'smartcar', 'tesla', 'none');
  END IF;

  -- 2) If the vehicles.remote_mgmt column exists, migrate values & change type
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_name = 'vehicles'
      AND column_name = 'remote_mgmt'
      AND udt_name = 'remote_mgmt_enum'
  ) THEN
    -- Drop default to allow type change when reverting enum
    ALTER TABLE vehicles
      ALTER COLUMN remote_mgmt DROP DEFAULT;

    -- Map any 'geotab' entries to a safe fallback
    UPDATE vehicles SET remote_mgmt = 'none' WHERE remote_mgmt::text = 'geotab';

    -- Switch the column to the old (temporary) enum
    ALTER TABLE vehicles
      ALTER COLUMN remote_mgmt TYPE remote_mgmt_enum_old
      USING remote_mgmt::text::remote_mgmt_enum_old;
  END IF;

  -- 3) Drop the enum that contains 'geotab' and rename the old one back
  DROP TYPE remote_mgmt_enum;
  ALTER TYPE remote_mgmt_enum_old RENAME TO remote_mgmt_enum;
END
$$;