-- Down migration for 0003_password_reset_tokens.sql

DROP TABLE IF EXISTS password_reset_tokens CASCADE;
