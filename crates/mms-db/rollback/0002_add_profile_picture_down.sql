-- Down migration for 0002_add_profile_picture.sql

ALTER TABLE users
DROP COLUMN IF EXISTS profile_picture_url;
