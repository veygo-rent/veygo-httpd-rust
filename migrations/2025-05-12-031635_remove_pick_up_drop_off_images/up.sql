ALTER TABLE agreements
DROP
COLUMN IF EXISTS pickup_front_image,
DROP
COLUMN IF EXISTS pickup_back_image,
    DROP
COLUMN IF EXISTS pickup_left_image,
    DROP
COLUMN IF EXISTS pickup_right_image,
    DROP
COLUMN IF EXISTS drop_off_front_image,
    DROP
COLUMN IF EXISTS drop_off_back_image,
    DROP
COLUMN IF EXISTS drop_off_left_image,
    DROP
COLUMN IF EXISTS drop_off_right_image;
