-- Your SQL goes here
-- up.sql

CREATE TYPE agreement_status_enum AS ENUM ('Rental', 'Void', 'Canceled');

CREATE TABLE agreements
(
    id                        SERIAL PRIMARY KEY,
    confirmation              VARCHAR                  NOT NULL UNIQUE,
    status                    agreement_status_enum    NOT NULL,
    user_name                 VARCHAR                  NOT NULL,
    user_date_of_birth        DATE                     NOT NULL,
    user_email                VARCHAR                  NOT NULL,
    user_phone                VARCHAR                  NOT NULL,
    user_billing_address      VARCHAR                  NOT NULL,
    rsvp_pickup_time          TIMESTAMP WITH TIME ZONE NOT NULL,
    rsvp_drop_off_time        TIMESTAMP WITH TIME ZONE NOT NULL,
    liability_protection_rate DOUBLE PRECISION         NOT NULL,
    pcdw_protection_rate      DOUBLE PRECISION         NOT NULL,
    pcdw_ext_protection_rate  DOUBLE PRECISION         NOT NULL,
    rsa_protection_rate       DOUBLE PRECISION         NOT NULL,
    pai_protection_rate       DOUBLE PRECISION         NOT NULL,
    actual_pickup_time        TIMESTAMP WITH TIME ZONE,
    pickup_odometer           INTEGER,
    pickup_level              INTEGER,
    pickup_front_image        VARCHAR,
    pickup_back_image         VARCHAR,
    pickup_left_image         VARCHAR,
    pickup_right_image        VARCHAR,
    actual_drop_off_time      TIMESTAMP WITH TIME ZONE,
    drop_off_odometer         INTEGER,
    drop_off_level            INTEGER,
    drop_off_front_image      VARCHAR,
    drop_off_back_image       VARCHAR,
    drop_off_left_image       VARCHAR,
    drop_off_right_image      VARCHAR,
    tax_rate                  DOUBLE PRECISION         NOT NULL,
    msrp_factor               DOUBLE PRECISION         NOT NULL,
    duration_rate             DOUBLE PRECISION         NOT NULL,
    apartment_id              INTEGER                  NOT NULL REFERENCES apartments (id),
    vehicle_id                INTEGER                  NOT NULL REFERENCES vehicles (id),
    renter_id                 INTEGER                  NOT NULL REFERENCES renters (id),
    payment_method_id         INTEGER                  NOT NULL REFERENCES payment_methods (id)
    -- Consider adding a constraint to ensure actual_pickup_time < actual_drop_off_time
);