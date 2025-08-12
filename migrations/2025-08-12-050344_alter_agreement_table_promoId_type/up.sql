ALTER TABLE agreements
ALTER COLUMN promo_id TYPE VARCHAR
USING promo_id::varchar;