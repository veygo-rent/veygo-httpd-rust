-- Your SQL goes here
CREATE TABLE do_not_rent_lists
(
    id    SERIAL PRIMARY KEY,
    name  VARCHAR UNIQUE,
    phone VARCHAR UNIQUE,
    email VARCHAR UNIQUE,
    note  TEXT NOT NULL,
    exp   DATE
);