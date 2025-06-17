CREATE TABLE taxes
(
    id           SERIAL PRIMARY KEY,
    name         VARCHAR          NOT NULL,
    multiplier   DOUBLE PRECISION NOT NULL,
    is_effective BOOLEAN          NOT NULL
);

INSERT INTO taxes (name, multiplier, is_effective)
VALUES ('IN Sales Tax', 0.07, true),
       ('IN Car Rental Excise Tax', 0.04, true);

ALTER TABLE apartments
    ADD COLUMN taxes INT[] NOT NULL DEFAULT ARRAY[]::INT[];

UPDATE apartments
SET taxes = ARRAY[1, 2]
WHERE name = 'Purdue University';

ALTER TABLE apartments
DROP
COLUMN IF EXISTS sales_tax_rate;

ALTER TABLE agreements
    ADD COLUMN taxes INT[] NOT NULL DEFAULT ARRAY[]::INT[];

ALTER TABLE agreements
DROP
COLUMN IF EXISTS tax_rate;

ALTER TABLE charges
    ADD COLUMN taxes INT[] NOT NULL DEFAULT ARRAY[]::INT[];

