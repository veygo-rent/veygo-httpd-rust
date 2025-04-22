-- 1) Add a fixed‐length checksum column
ALTER TABLE charges
    ADD COLUMN checksum VARCHAR NOT NULL,
ADD COLUMN transponder_company_id INTEGER,
ADD COLUMN vehicle_identifier VARCHAR;

-- 2) Back‑fill existing rows (if any) with a placeholder or compute via SQL/your app:
--    You can either leave them as all‑zeros and re‑compute on next update,
--    or run a one‑off UPDATE with your Rust app or an SQL md5 expression.

-- 3) Add a unique index so Postgres will reject duplicates
CREATE UNIQUE INDEX idx_charges_checksum
    ON charges (checksum);