-- Your SQL goes here
ALTER TABLE transponder_companies
    ADD COLUMN timestamp_format VARCHAR NOT NULL,
    ADD COLUMN timezone VARCHAR;