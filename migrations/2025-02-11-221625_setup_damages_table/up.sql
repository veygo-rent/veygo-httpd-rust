-- Your SQL goes here
-- up.sql
CREATE TABLE damages
(
    id                                 SERIAL PRIMARY KEY,
    note                               TEXT                     NOT NULL,
    record_date                        TIMESTAMP WITH TIME ZONE NOT NULL,
    occur_date                         TIMESTAMP WITH TIME ZONE NOT NULL,
    standard_coordination_x_percentage INTEGER                  NOT NULL,
    standard_coordination_y_percentage INTEGER                  NOT NULL,
    first_image                        VARCHAR,
    second_image                       VARCHAR,
    third_image                        VARCHAR,
    fourth_image                       VARCHAR,                           -- Consider renaming to "fourth_image"
    fixed_date                         TIMESTAMP WITH TIME ZONE,
    fixed_amount                       DOUBLE PRECISION,
    agreement_id                       INTEGER REFERENCES agreements (id) -- Allow NULL (Option<i32>)
);