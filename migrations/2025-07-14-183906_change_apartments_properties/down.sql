-- This file should undo anything in `up.sql`

-- Re-add UNIQUE constraints on email and phone
ALTER TABLE apartments
    ADD CONSTRAINT apartments_email_key UNIQUE (email);
ALTER TABLE apartments
    ADD CONSTRAINT apartments_phone_key UNIQUE (phone);
