-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods
    ADD CONSTRAINT payment_methods_md5_key UNIQUE (md5);