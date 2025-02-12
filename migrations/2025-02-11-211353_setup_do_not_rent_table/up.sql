-- Your SQL goes here
CREATE TABLE do_not_rent_lists
(
    id    SERIAL PRIMARY KEY,
    name  VARCHAR,
    phone VARCHAR,
    email VARCHAR,
    note  TEXT NOT NULL,
    exp   DATE
);