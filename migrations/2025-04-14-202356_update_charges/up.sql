-- Make `agreement_id` optional
ALTER TABLE charges
    ALTER COLUMN agreement_id DROP NOT NULL;