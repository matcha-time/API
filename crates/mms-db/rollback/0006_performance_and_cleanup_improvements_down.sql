-- Down migration for 0006_performance_and_cleanup_improvements.sql

-- ============================================================================
-- PART 1: DROP TRIGGERS
-- ============================================================================

DROP TRIGGER IF EXISTS trigger_cleanup_password_reset_tokens
    ON password_reset_tokens;

DROP TRIGGER IF EXISTS trigger_cleanup_email_verification_tokens
    ON email_verification_tokens;

DROP TRIGGER IF EXISTS trigger_cleanup_refresh_tokens
    ON refresh_tokens;

-- ============================================================================
-- PART 2: DROP FUNCTIONS
-- ============================================================================

DROP FUNCTION IF EXISTS trigger_cleanup_expired_tokens();
DROP FUNCTION IF EXISTS cleanup_all_expired_tokens();
DROP FUNCTION IF EXISTS cleanup_expired_refresh_tokens();
DROP FUNCTION IF EXISTS cleanup_expired_email_verification_tokens();
DROP FUNCTION IF EXISTS cleanup_expired_password_reset_tokens();
DROP FUNCTION IF EXISTS refresh_stale_deck_progress(UUID);

-- ============================================================================
-- PART 3: DROP INDEXES
-- ============================================================================

DROP INDEX IF EXISTS idx_refresh_tokens_expires_at;
DROP INDEX IF EXISTS idx_email_verification_tokens_expires_at;
DROP INDEX IF EXISTS idx_password_reset_tokens_expires_at;
DROP INDEX IF EXISTS idx_email_verification_tokens_user_id;
DROP INDEX IF EXISTS idx_password_reset_tokens_user_id;
DROP INDEX IF EXISTS idx_users_email;
