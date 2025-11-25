-- Migration 0006: Performance and Cleanup Improvements
-- This migration addresses:
-- 1. Missing critical indexes (users.email, token tables)
-- 2. Automatic token cleanup mechanisms
-- 3. Refresh token cleanup scheduling

-- ============================================================================
-- PART 1: MISSING INDEXES FOR PERFORMANCE
-- ============================================================================

-- Critical: Index on users.email for login performance
-- This is queried on EVERY login request - must be indexed!
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Index for password reset token lookups
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user_id
    ON password_reset_tokens(user_id);

-- Index for email verification token lookups
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_user_id
    ON email_verification_tokens(user_id);

-- Index for efficient token expiration cleanup queries
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_expires_at
    ON password_reset_tokens(expires_at)
    WHERE used_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_expires_at
    ON email_verification_tokens(expires_at)
    WHERE used_at IS NULL;

-- Index for efficient refresh token cleanup
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at
    ON refresh_tokens(expires_at);

-- ============================================================================
-- PART 2: AUTOMATIC TOKEN CLEANUP FUNCTIONS
-- ============================================================================

-- Function to clean up expired password reset tokens
CREATE OR REPLACE FUNCTION cleanup_expired_password_reset_tokens()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- Delete tokens that are either expired or already used
    DELETE FROM password_reset_tokens
    WHERE expires_at < NOW() OR used_at IS NOT NULL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up expired email verification tokens
CREATE OR REPLACE FUNCTION cleanup_expired_email_verification_tokens()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- Delete tokens that are either expired or already used
    DELETE FROM email_verification_tokens
    WHERE expires_at < NOW() OR used_at IS NOT NULL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up expired refresh tokens
CREATE OR REPLACE FUNCTION cleanup_expired_refresh_tokens()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- Delete tokens that have expired
    DELETE FROM refresh_tokens
    WHERE expires_at < NOW();

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Master cleanup function that runs all cleanup tasks
CREATE OR REPLACE FUNCTION cleanup_all_expired_tokens()
RETURNS TABLE(
    password_reset_cleaned INTEGER,
    email_verification_cleaned INTEGER,
    refresh_tokens_cleaned INTEGER,
    total_cleaned INTEGER
) AS $$
DECLARE
    pr_count INTEGER;
    ev_count INTEGER;
    rt_count INTEGER;
BEGIN
    pr_count := cleanup_expired_password_reset_tokens();
    ev_count := cleanup_expired_email_verification_tokens();
    rt_count := cleanup_expired_refresh_tokens();

    RETURN QUERY SELECT
        pr_count,
        ev_count,
        rt_count,
        (pr_count + ev_count + rt_count);
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 3: AUTOMATIC TRIGGER FOR IMMEDIATE CLEANUP ON INSERT
-- ============================================================================

-- Trigger function to cleanup old tokens whenever new ones are created
-- This prevents unbounded growth by doing opportunistic cleanup
CREATE OR REPLACE FUNCTION trigger_cleanup_expired_tokens()
RETURNS TRIGGER AS $$
BEGIN
    -- Clean up expired tokens from the same table (opportunistic cleanup)
    IF TG_TABLE_NAME = 'password_reset_tokens' THEN
        DELETE FROM password_reset_tokens
        WHERE expires_at < NOW() OR used_at IS NOT NULL;
    ELSIF TG_TABLE_NAME = 'email_verification_tokens' THEN
        DELETE FROM email_verification_tokens
        WHERE expires_at < NOW() OR used_at IS NOT NULL;
    ELSIF TG_TABLE_NAME = 'refresh_tokens' THEN
        DELETE FROM refresh_tokens
        WHERE expires_at < NOW();
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply triggers to token tables
CREATE TRIGGER trigger_cleanup_password_reset_tokens
    AFTER INSERT ON password_reset_tokens
    FOR EACH STATEMENT
    EXECUTE FUNCTION trigger_cleanup_expired_tokens();

CREATE TRIGGER trigger_cleanup_email_verification_tokens
    AFTER INSERT ON email_verification_tokens
    FOR EACH STATEMENT
    EXECUTE FUNCTION trigger_cleanup_expired_tokens();

CREATE TRIGGER trigger_cleanup_refresh_tokens
    AFTER INSERT ON refresh_tokens
    FOR EACH STATEMENT
    EXECUTE FUNCTION trigger_cleanup_expired_tokens();

-- ============================================================================
-- PART 4: FUNCTION TO REFRESH STALE DECK PROGRESS (cards_due_today)
-- ============================================================================

-- Function to refresh all stale deck progress for a user
-- This recalculates cards_due_today for decks that haven't been updated recently
CREATE OR REPLACE FUNCTION refresh_stale_deck_progress(p_user_id UUID)
RETURNS INTEGER AS $$
DECLARE
    updated_count INTEGER;
    deck_record RECORD;
BEGIN
    -- For each deck with potentially stale data (not updated in last hour)
    FOR deck_record IN
        SELECT deck_id
        FROM user_deck_progress
        WHERE user_id = p_user_id
        AND (updated_at < NOW() - INTERVAL '1 hour' OR updated_at IS NULL)
    LOOP
        PERFORM refresh_deck_progress(p_user_id, deck_record.deck_id);
    END LOOP;

    GET DIAGNOSTICS updated_count = ROW_COUNT;
    RETURN updated_count;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- COMMENTS FOR MAINTENANCE
-- ============================================================================

COMMENT ON FUNCTION cleanup_expired_password_reset_tokens() IS
'Removes expired and used password reset tokens. Can be called manually or via cron job.';

COMMENT ON FUNCTION cleanup_expired_email_verification_tokens() IS
'Removes expired and used email verification tokens. Can be called manually or via cron job.';

COMMENT ON FUNCTION cleanup_expired_refresh_tokens() IS
'Removes expired refresh tokens. Can be called manually or via cron job.';

COMMENT ON FUNCTION cleanup_all_expired_tokens() IS
'Master cleanup function that removes all expired tokens and returns statistics.
Recommended to run daily via cron job or application scheduler.';

COMMENT ON FUNCTION refresh_stale_deck_progress(UUID) IS
'Refreshes deck progress statistics that haven''t been updated in the last hour.
Call this when loading a user''s dashboard to ensure cards_due_today is accurate.';
