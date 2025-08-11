ALTER TABLE agreements
ADD COLUMN apartment_id INTEGER REFERENCES apartments(id);
