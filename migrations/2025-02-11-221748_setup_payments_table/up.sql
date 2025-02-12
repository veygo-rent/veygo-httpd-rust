-- Your SQL goes here
-- up.sql
CREATE TYPE payment_type_enum AS ENUM ('Cash', 'ACH', 'CC', 'BadDebt');

CREATE TABLE payments
(
    id                SERIAL PRIMARY KEY,
    payment_type      payment_type_enum        NOT NULL,      -- Use the custom enum type
    time              TIMESTAMP WITH TIME ZONE NOT NULL,
    amount            DOUBLE PRECISION         NOT NULL,
    note              VARCHAR,
    reference_number  VARCHAR,
    agreement_id      INTEGER                  NOT NULL REFERENCES agreements (id),
    payment_method_id INTEGER REFERENCES payment_methods (id) -- Optional FK
);