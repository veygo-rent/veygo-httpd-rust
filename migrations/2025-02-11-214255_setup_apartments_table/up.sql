-- Your SQL goes here
CREATE TABLE apartments
(
    id                           SERIAL PRIMARY KEY,
    name                         VARCHAR          NOT NULL,
    email                        VARCHAR          NOT NULL,
    phone                        VARCHAR          NOT NULL,
    address                      VARCHAR          NOT NULL,
    accepted_school_email_domain VARCHAR          NOT NULL,
    free_tier_hours              DOUBLE PRECISION NOT NULL,
    free_tier_rate               DOUBLE PRECISION NOT NULL,
    silver_tier_hours            DOUBLE PRECISION NOT NULL,
    silver_tier_rate             DOUBLE PRECISION NOT NULL,
    gold_tier_hours              DOUBLE PRECISION NOT NULL,
    gold_tier_rate               DOUBLE PRECISION NOT NULL,
    platinum_tier_hours          DOUBLE PRECISION NOT NULL,
    platinum_tier_rate           DOUBLE PRECISION NOT NULL,
    duration_rate                DOUBLE PRECISION NOT NULL,
    liability_protection_rate    DOUBLE PRECISION NOT NULL,
    pcdw_protection_rate         DOUBLE PRECISION NOT NULL,
    pcdw_ext_protection_rate     DOUBLE PRECISION NOT NULL,
    rsa_protection_rate          DOUBLE PRECISION NOT NULL,
    pai_protection_rate          DOUBLE PRECISION NOT NULL,
    sales_tax_rate               DOUBLE PRECISION NOT NULL,
    is_operating                 BOOLEAN          NOT NULL,
    is_public                    BOOLEAN          NOT NULL
);