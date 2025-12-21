-- Migration: Add progress_percentage to user_deck_progress table
-- This allows tracking deck completion based on a points system (max 10 points per card)
-- Progress formula: SUM(GREATEST(0, times_correct - times_wrong)) / (total_cards * 10) * 100

-- Add the progress_percentage column
ALTER TABLE user_deck_progress
ADD COLUMN progress_percentage DECIMAL(5,2) NOT NULL DEFAULT 0.00;

-- Update the refresh_deck_progress function to calculate progress percentage
CREATE OR REPLACE FUNCTION refresh_deck_progress(p_user_id UUID, p_deck_id UUID)
RETURNS void AS $$
DECLARE
    v_total_cards INT;
    v_current_points INT;
    v_max_points INT;
    v_progress_percentage DECIMAL(5,2);
BEGIN
    -- Calculate statistics
    SELECT
        COUNT(*) as total_cards,
        COALESCE(SUM(GREATEST(0, COALESCE(ucp.times_correct, 0) - COALESCE(ucp.times_wrong, 0))), 0) as current_points
    INTO v_total_cards, v_current_points
    FROM deck_flashcards df
    LEFT JOIN user_card_progress ucp
        ON df.flashcard_id = ucp.flashcard_id
        AND ucp.user_id = p_user_id
    WHERE df.deck_id = p_deck_id;

    -- Calculate max points (10 per card) and progress percentage
    v_max_points := v_total_cards * 10;
    v_progress_percentage := CASE
        WHEN v_max_points > 0 THEN LEAST(100.00, (v_current_points::DECIMAL / v_max_points * 100))
        ELSE 0.00
    END;

    -- Insert or update deck progress
    INSERT INTO user_deck_progress (
        user_id, deck_id, total_cards, mastered_cards,
        cards_due_today, total_practices, progress_percentage, last_practiced_at, updated_at
    )
    SELECT
        p_user_id,
        p_deck_id,
        COUNT(*) as total_cards,
        COUNT(*) FILTER (WHERE ucp.mastered_at IS NOT NULL) as mastered_cards,
        COUNT(*) FILTER (WHERE ucp.next_review_at <= NOW()) as cards_due_today,
        COALESCE(SUM(ucp.times_correct + ucp.times_wrong), 0) as total_practices,
        v_progress_percentage,
        MAX(ucp.last_review_at) as last_practiced_at,
        NOW()
    FROM deck_flashcards df
    LEFT JOIN user_card_progress ucp
        ON df.flashcard_id = ucp.flashcard_id
        AND ucp.user_id = p_user_id
    WHERE df.deck_id = p_deck_id
    ON CONFLICT (user_id, deck_id)
    DO UPDATE SET
        total_cards = EXCLUDED.total_cards,
        mastered_cards = EXCLUDED.mastered_cards,
        cards_due_today = EXCLUDED.cards_due_today,
        total_practices = EXCLUDED.total_practices,
        progress_percentage = EXCLUDED.progress_percentage,
        last_practiced_at = EXCLUDED.last_practiced_at,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- Backfill progress_percentage for existing data
-- This recalculates progress for all existing user_deck_progress records
DO $$
DECLARE
    deck_record RECORD;
BEGIN
    FOR deck_record IN
        SELECT DISTINCT user_id, deck_id
        FROM user_deck_progress
    LOOP
        PERFORM refresh_deck_progress(deck_record.user_id, deck_record.deck_id);
    END LOOP;
END $$;

-- Add comment for documentation
COMMENT ON COLUMN user_deck_progress.progress_percentage IS 'Deck completion percentage based on points system: each card can contribute 0-10 points (max(0, times_correct - times_wrong)), progress = (sum of card points) / (total_cards * 10) * 100';
