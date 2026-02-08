-- Migration: Add automatic updated_at trigger
-- Ensures updated_at is always set on UPDATE, even if application code forgets.
-- This is a safety net — application code can still set it explicitly, but the
-- trigger guarantees it's never stale.

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_user_card_progress_updated_at
    BEFORE UPDATE ON user_card_progress
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_user_deck_progress_updated_at
    BEFORE UPDATE ON user_deck_progress
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_user_stats_updated_at
    BEFORE UPDATE ON user_stats
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
