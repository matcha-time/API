-- Streak calculation function
--
-- Computes the current streak (consecutive days with reviews ending at today
-- or yesterday) and updates user_stats accordingly.
-- Called after recording activity during a review submission.

CREATE OR REPLACE FUNCTION calculate_and_update_streak(p_user_id UUID)
RETURNS void AS $$
DECLARE
    v_streak INT := 0;
    v_activity_date DATE;
    v_expected_date DATE;
BEGIN
    -- Start from today: if user reviewed today, that's the anchor.
    -- If not, check yesterday (streak is still alive but user hasn't reviewed yet today).
    v_expected_date := CURRENT_DATE;

    FOR v_activity_date IN
        SELECT activity_date
        FROM user_activity
        WHERE user_id = p_user_id
          AND activity_date <= CURRENT_DATE
        ORDER BY activity_date DESC
    LOOP
        IF v_activity_date = v_expected_date THEN
            -- Consecutive day found
            v_streak := v_streak + 1;
            v_expected_date := v_expected_date - 1;
        ELSIF v_streak = 0 AND v_activity_date = CURRENT_DATE - 1 THEN
            -- No activity today, but yesterday counts as alive
            v_streak := 1;
            v_expected_date := v_activity_date - 1;
        ELSE
            -- Gap found, stop counting
            EXIT;
        END IF;
    END LOOP;

    UPDATE user_stats
    SET current_streak_days = v_streak,
        longest_streak_days = GREATEST(longest_streak_days, v_streak),
        updated_at = NOW()
    WHERE user_id = p_user_id;
END;
$$ LANGUAGE plpgsql;
