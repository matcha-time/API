-- Password reset tokens table
CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash    TEXT NOT NULL UNIQUE,
    expires_at    TIMESTAMPTZ NOT NULL,
    used_at       TIMESTAMPTZ,
    created_at    TIMESTAMPTZ DEFAULT NOW(),

    -- Ensure only one active token per user
    CONSTRAINT one_active_token_per_user UNIQUE (user_id, used_at)
        WHERE used_at IS NULL
);

-- Index for fast token lookup
CREATE INDEX IF NOT EXISTS idx_reset_tokens_hash ON password_reset_tokens(token_hash)
    WHERE used_at IS NULL AND expires_at > NOW();

-- Index for cleanup of expired tokens
CREATE INDEX IF NOT EXISTS idx_reset_tokens_expires ON password_reset_tokens(expires_at);
