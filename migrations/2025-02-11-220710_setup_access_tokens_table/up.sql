-- Your SQL goes here
CREATE TABLE access_tokens
(
    id      SERIAL PRIMARY KEY,
    user_id INTEGER                  NOT NULL REFERENCES renters (id),
    token   VARCHAR                  NOT NULL,
    exp     TIMESTAMP WITH TIME ZONE NOT NULL
);