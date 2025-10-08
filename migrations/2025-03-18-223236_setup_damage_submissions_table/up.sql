-- Your SQL goes here
CREATE TABLE damage_submissions
(
    id           SERIAL PRIMARY KEY,
    reported_by  INTEGER NOT NULL REFERENCES renters (id),
    first_image  VARCHAR NOT NULL,
    second_image VARCHAR NOT NULL,
    third_image  VARCHAR,
    fourth_image VARCHAR,
    description  VARCHAR NOT NULL,
    processed    BOOLEAN NOT NULL DEFAULT false
);