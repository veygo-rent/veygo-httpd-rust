-- Your SQL goes here
-- up.sql
CREATE TABLE charges
(
    id           SERIAL PRIMARY KEY,
    name         VARCHAR                  NOT NULL,
    time         TIMESTAMP WITH TIME ZONE NOT NULL,
    amount       DOUBLE PRECISION         NOT NULL,
    note         VARCHAR,
    agreement_id INTEGER                  NOT NULL REFERENCES agreements (id)
);