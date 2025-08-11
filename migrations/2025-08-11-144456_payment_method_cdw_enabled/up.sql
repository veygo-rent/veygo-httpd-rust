ALTER TABLE payment_methods
    ADD COLUMN cdw_enabled BOOLEAN NOT NULL DEFAULT false;