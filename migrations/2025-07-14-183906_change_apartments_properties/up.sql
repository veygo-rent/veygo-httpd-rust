-- Drop existing UNIQUE constraints on email and phone if they exist
ALTER TABLE apartments DROP CONSTRAINT IF EXISTS apartments_email_key;
ALTER TABLE apartments DROP CONSTRAINT IF EXISTS apartments_phone_key;