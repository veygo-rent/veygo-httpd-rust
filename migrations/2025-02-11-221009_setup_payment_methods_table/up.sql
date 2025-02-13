-- Your SQL goes here
-- up.sql
CREATE TABLE payment_methods
(
    id                  SERIAL PRIMARY KEY,
    cardholder_name     VARCHAR NOT NULL,
    masked_card_number  VARCHAR NOT NULL,
    network             VARCHAR NOT NULL,
    expiration          VARCHAR NOT NULL,
    token               VARCHAR NOT NULL UNIQUE,
    nickname            VARCHAR,
    is_enabled          BOOLEAN NOT NULL,
    renter_id           INTEGER NOT NULL REFERENCES renters (id),
    last_used_date_time TIMESTAMP WITH TIME ZONE
);