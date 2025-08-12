ALTER TABLE agreements
    ALTER COLUMN promo_id TYPE INTEGER
    USING promo_id::integer;
