UPDATE apartments
SET uni_id = 0
WHERE name = 'Purdue University';

ALTER TABLE apartments
    ALTER COLUMN uni_id SET DEFAULT 0;
