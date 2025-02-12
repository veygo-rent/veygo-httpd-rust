-- Your SQL goes here
CREATE TABLE transponder_companies
(
    id                                       SERIAL PRIMARY KEY,
    name                                     VARCHAR NOT NULL,
    corresponding_key_for_vehicle_id         VARCHAR NOT NULL,
    corresponding_key_for_transaction_name   VARCHAR NOT NULL,
    custom_prefix_for_transaction_name       VARCHAR NOT NULL,
    corresponding_key_for_transaction_time   VARCHAR NOT NULL,
    corresponding_key_for_transaction_amount VARCHAR NOT NULL
);