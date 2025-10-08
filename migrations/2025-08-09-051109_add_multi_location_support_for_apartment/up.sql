CREATE TYPE remote_mgmt_enum AS ENUM ('revers', 'smartcar', 'tesla', 'none');

CREATE TABLE locations
(
    id           SERIAL PRIMARY KEY,
    apartment_id INT              NOT NULL REFERENCES apartments (id),
    name         VARCHAR(255)     NOT NULL,
    description  TEXT,
    latitude     DOUBLE PRECISION NOT NULL,
    longitude    DOUBLE PRECISION NOT NULL,
    enabled      BOOLEAN          NOT NULL DEFAULT true
);

ALTER TABLE vehicles
DROP COLUMN apartment_id,
ADD COLUMN location_id INT NOT NULL REFERENCES locations(id),
ADD COLUMN remote_mgmt remote_mgmt_enum NOT NULL DEFAULT 'none';

ALTER TABLE agreements
ADD COLUMN location_id INT NOT NULL REFERENCES locations(id);