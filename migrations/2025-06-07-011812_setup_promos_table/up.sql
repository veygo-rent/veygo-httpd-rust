CREATE TABLE promos
(
    code        VARCHAR PRIMARY KEY      NOT NULL,
    name        VARCHAR                  NOT NULL,
    amount      DOUBLE PRECISION         NOT NULL,
    is_enabled  BOOLEAN                  NOT NULL DEFAULT true,
    is_one_time BOOLEAN                  NOT NULL DEFAULT false,
    exp         TIMESTAMP WITH TIME ZONE NOT NULL,
    user_id     INTEGER                  NOT NULL DEFAULT 0,
    apt_id      INTEGER                  NOT NULL DEFAULT 0,
    uni_id      INTEGER                  NOT NULL DEFAULT 0
);
ALTER TABLE agreements
    ADD COLUMN promo_id INTEGER;