-- Your SQL goes here
CREATE TABLE access_tokens
(
    id      SERIAL PRIMARY KEY,
    user_id INTEGER                  NOT NULL REFERENCES renters (id),
    token   BYTEA                    NOT NULL UNIQUE,
    exp     TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (CURRENT_TIMESTAMP AT TIME ZONE 'UTC' + INTERVAL '28 days')
);