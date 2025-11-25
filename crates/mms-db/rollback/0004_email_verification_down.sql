-- Down migration for 0004_email_verification.sql

-- Drop the email verification tokens table
DROP TABLE IF EXISTS email_verification_tokens CASCADE;

-- Remove the email_verified column from users table
ALTER TABLE users
DROP COLUMN IF EXISTS email_verified;
