-- Your SQL goes here
CREATE TYPE gender_enum AS ENUM ('Male', 'Female', 'Other', 'PNTS'); -- Example ENUM
CREATE TYPE plan_tier_enum AS ENUM ('Free', 'Silver', 'Gold', 'Platinum');
CREATE TYPE employee_tier_enum AS ENUM ('User', 'GeneralEmployee', 'Maintenance', 'Admin');

CREATE TABLE renters
(
    id                              SERIAL PRIMARY KEY,
    name                            VARCHAR                  NOT NULL,
    student_email                   VARCHAR                  NOT NULL UNIQUE,
    student_email_expiration        DATE,
    password                        VARCHAR                  NOT NULL,
    phone                           VARCHAR                  NOT NULL UNIQUE,
    phone_is_verified               BOOLEAN                  NOT NULL DEFAULT FALSE,
    date_of_birth                   DATE                     NOT NULL,
    profile_picture                 VARCHAR,
    gender                          gender_enum,                                                  -- Use the custom enum
    date_of_registration            TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    drivers_license_number          VARCHAR,
    drivers_license_state_region    VARCHAR,
    drivers_license_image           VARCHAR,
    drivers_license_image_secondary VARCHAR,
    drivers_license_expiration      DATE,
    insurance_id_image              VARCHAR,
    insurance_id_expiration         DATE,
    lease_agreement_image           VARCHAR,
    apartment_id                    INTEGER                  NOT NULL REFERENCES apartments (id), -- FOREIGN KEY!
    lease_agreement_expiration      DATE,
    billing_address                 VARCHAR,
    signature_image                 VARCHAR,
    signature_datetime              TIMESTAMP WITH TIME ZONE,
    plan_tier                       plan_tier_enum           NOT NULL DEFAULT 'Free',
    plan_renewal_day                VARCHAR                  NOT NULL,
    plan_expire_month_year          VARCHAR                  NOT NULL,
    plan_available_duration         DOUBLE PRECISION         NOT NULL,
    is_plan_annual                  BOOLEAN                  NOT NULL DEFAULT false,
    employee_tier                   employee_tier_enum       NOT NULL DEFAULT 'User'
);