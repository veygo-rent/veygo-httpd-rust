-- Your SQL goes here
-- up.sql
CREATE TABLE vehicles
(
    id                            SERIAL PRIMARY KEY,
    vin                           VARCHAR          NOT NULL UNIQUE,
    name                          VARCHAR          NOT NULL,
    available                     BOOLEAN          NOT NULL,
    license_number                VARCHAR          NOT NULL,
    license_state                 VARCHAR          NOT NULL,
    year                          VARCHAR          NOT NULL, -- Consider using SMALLINT or INTEGER
    make                          VARCHAR          NOT NULL,
    model                         VARCHAR          NOT NULL,
    msrp_factor                   DOUBLE PRECISION NOT NULL,
    image_link                    VARCHAR,
    odometer                      INTEGER          NOT NULL,
    tank_size                     DOUBLE PRECISION NOT NULL,
    tank_level_percentage         INTEGER          NOT NULL,
    first_transponder_number      VARCHAR,
    first_transponder_company_id  INTEGER,
    second_transponder_number     VARCHAR,
    second_transponder_company_id INTEGER,
    third_transponder_number      VARCHAR,
    third_transponder_company_id  INTEGER,
    fourth_transponder_number     VARCHAR,                   -- "forth" -> "fourth"
    fourth_transponder_company_id INTEGER,
    apartment_id                  INTEGER          NOT NULL REFERENCES apartments (id)
);