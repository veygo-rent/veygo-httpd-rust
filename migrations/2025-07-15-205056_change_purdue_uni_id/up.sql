UPDATE apartments
SET uni_id = 1
WHERE name = 'Purdue University';

ALTER TABLE apartments
    ALTER COLUMN uni_id DROP DEFAULT;