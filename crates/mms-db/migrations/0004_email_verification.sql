-- Add email verification functionality

-- Add email_verified column to users table
ALTER TABLE users
ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT FALSE;

-- Users who signed up via Google OAuth should be considered verified
UPDATE users
SET email_verified = TRUE
WHERE auth_provider = 'google';

-- Create email verification tokens table
CREATE TABLE email_verification_tokens (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash    TEXT NOT NULL UNIQUE,
    expires_at    TIMESTAMPTZ NOT NULL,
    used_at       TIMESTAMPTZ,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);
