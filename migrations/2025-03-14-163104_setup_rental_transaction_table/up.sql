-- Your SQL goes here
CREATE TYPE transaction_type_enum AS ENUM ('credit', 'cash');

CREATE TABLE rental_transactions
(
    id               SERIAL PRIMARY KEY,
    agreement_id     INTEGER                  NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    transaction_type transaction_type_enum    NOT NULL,
    duration         FLOAT                    NOT NULL,
    transaction_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);