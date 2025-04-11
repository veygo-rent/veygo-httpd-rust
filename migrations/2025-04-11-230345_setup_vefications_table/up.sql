-- Your SQL goes here
CREATE TYPE verification_type_enum AS ENUM ('email', 'phone');

CREATE TABLE verifications
(
    id                  SERIAL PRIMARY KEY,
    verification_method verification_type_enum   NOT NULL,
    renter_id           INTEGER                  NOT NULL REFERENCES renters (id),
    expires_at          TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (CURRENT_TIMESTAMP + INTERVAL '10 minutes'),
    code                VARCHAR                  NOT NULL
);
