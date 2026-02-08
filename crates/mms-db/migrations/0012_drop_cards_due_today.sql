-- Migration: Remove cards_due_today from user_deck_progress
--
-- This cached column goes stale immediately after being written (it's a point-in-time
-- snapshot, not a daily count). The roadmap queries already compute it on-the-fly via
-- a subquery against user_card_progress.next_review_at, so the cached value is redundant.

-- Rewrite refresh_deck_progress without the cards_due_today column
CREATE OR REPLACE FUNCTION refresh_deck_progress(p_user_id UUID, p_deck_id UUID, p_mastery_threshold INT DEFAULT 10)
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

    -- Calculate max points and progress percentage using the parameterized threshold
    v_max_points := v_total_cards * p_mastery_threshold;
    v_progress_percentage := CASE
        WHEN v_max_points > 0 THEN LEAST(100.00, (v_current_points::DECIMAL / v_max_points * 100))
        ELSE 0.00
    END;

    -- Insert or update deck progress
    INSERT INTO user_deck_progress (
        user_id, deck_id, total_cards, mastered_cards,
        total_practices, progress_percentage, last_practiced_at, updated_at
    )
    SELECT
        p_user_id,
        p_deck_id,
        COUNT(*) as total_cards,
        COUNT(*) FILTER (WHERE ucp.mastered_at IS NOT NULL) as mastered_cards,
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
        total_practices = EXCLUDED.total_practices,
        progress_percentage = EXCLUDED.progress_percentage,
        last_practiced_at = EXCLUDED.last_practiced_at,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- Drop the stale cached column
ALTER TABLE user_deck_progress DROP COLUMN cards_due_today;
