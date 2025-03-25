-- Your SQL goes here
-- up.sql
CREATE TYPE payment_type_enum AS ENUM ('canceled', 'processing', 'requires_action', 'requires_capture', 'requires_confirmation', 'requires_payment_method', 'succeeded');

CREATE TABLE payments
(
    id                SERIAL PRIMARY KEY,
    payment_type      payment_type_enum        NOT NULL,
    time              TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    amount            DOUBLE PRECISION         NOT NULL,
    note              VARCHAR,
    reference_number  VARCHAR UNIQUE,
    agreement_id      INTEGER                  NOT NULL REFERENCES agreements (id),
    payment_method_id INTEGER REFERENCES payment_methods (id)
);