-- Refresh tokens table for secure session management
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,  -- SHA-256 hash of the refresh token
    device_info     TEXT,                  -- Optional: browser/device identifier
    ip_address      TEXT,                  -- Optional: IP address for security audit
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ DEFAULT NOW()
);

-- Fast lookup: find token by hash for refresh endpoint
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON refresh_tokens(token_hash);

-- Fast lookup: get all tokens for a user (for logout all devices)
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON refresh_tokens(user_id);
