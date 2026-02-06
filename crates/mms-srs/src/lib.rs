//! SRS (Spaced Repetition System) library for Matcha Time
//!
//! This crate provides the core spaced repetition algorithm and related functionality
//! for scheduling flashcard reviews.

use chrono::{DateTime, Duration, Utc};

/// Compute the next review date based on the SRS algorithm.
///
/// Uses an exponential interval system based on score (times_correct - times_wrong).
/// Designed for roadmap-based learning with aggressive early practice using hours,
/// then transitioning to days with exponential doubling.
///
/// # Arguments
///
/// * `times_correct` - Number of times the card was answered correctly
/// * `times_wrong` - Number of times the card was answered incorrectly
///
/// # Returns
///
/// The next review date as a `DateTime<Utc>`
///
/// # Algorithm
///
/// The algorithm calculates a score as `times_correct - times_wrong` and applies
/// exponential intervals with hour-based early learning:
///
/// * Score ≤ 0: 2 hours (immediate retry)
/// * Score 1: 4 hours
/// * Score 2: 8 hours
/// * Score 3: 1 day
/// * Score 4: 2 days
/// * Score 5: 5 days
/// * Score 6: 10 days
/// * Score 7: 20 days (~3 weeks)
/// * Score 8: 40 days (~6 weeks)
/// * Score 9: 60 days (2 months)
/// * Score ≥ 10: 90 days (3 months, mastered)
pub fn compute_next_review(times_correct: i32, times_wrong: i32) -> DateTime<Utc> {
    let score = times_correct - times_wrong;

    // SRS intervals with exponential doubling (hours → days)
    let interval = match score {
        s if s <= 0 => Duration::hours(2), // 2 hours - immediate retry
        1 => Duration::hours(4),           // 4 hours
        2 => Duration::hours(8),           // 8 hours
        3 => Duration::days(1),            // 1 day
        4 => Duration::days(2),            // 2 days
        5 => Duration::days(5),            // 5 days
        6 => Duration::days(10),           // 10 days
        7 => Duration::days(20),           // ~3 weeks
        8 => Duration::days(40),           // ~6 weeks
        9 => Duration::days(60),           // 2 months
        _ => Duration::days(90),           // 3 months (mastered)
    };

    Utc::now() + interval
}

/// Calculate the current SRS score for a card.
///
/// # Arguments
///
/// * `times_correct` - Number of times the card was answered correctly
/// * `times_wrong` - Number of times the card was answered incorrectly
///
/// # Returns
///
/// The score as an `i32` (times_correct - times_wrong)
pub fn calculate_score(times_correct: i32, times_wrong: i32) -> i32 {
    times_correct - times_wrong
}

/// Get the interval in hours for a given score.
///
/// # Arguments
///
/// * `score` - The SRS score (typically times_correct - times_wrong)
///
/// # Returns
///
/// The interval in hours as an `i64`
///
/// # Note
///
/// Returns intervals in hours for consistency across all score ranges.
/// Scores 0-2 use hour-based intervals, scores 3+ use day-based intervals
/// converted to hours for uniform return type.
pub fn get_interval_for_score(score: i32) -> i64 {
    match score {
        s if s <= 0 => 2, // 2 hours
        1 => 4,           // 4 hours
        2 => 8,           // 8 hours
        3 => 24,          // 1 day (24 hours)
        4 => 48,          // 2 days (48 hours)
        5 => 120,         // 5 days (120 hours)
        6 => 240,         // 10 days (240 hours)
        7 => 480,         // 20 days (480 hours)
        8 => 960,         // 40 days (960 hours)
        9 => 1440,        // 60 days (1440 hours)
        _ => 2160,        // 90 days (2160 hours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_score() {
        assert_eq!(calculate_score(5, 2), 3);
        assert_eq!(calculate_score(0, 0), 0);
        assert_eq!(calculate_score(2, 5), -3);
    }

    #[test]
    fn test_get_interval_for_score() {
        assert_eq!(get_interval_for_score(-1), 2); // 2 hours
        assert_eq!(get_interval_for_score(0), 2); // 2 hours
        assert_eq!(get_interval_for_score(1), 4); // 4 hours
        assert_eq!(get_interval_for_score(2), 8); // 8 hours
        assert_eq!(get_interval_for_score(3), 24); // 1 day (24h)
        assert_eq!(get_interval_for_score(4), 48); // 2 days (48h)
        assert_eq!(get_interval_for_score(5), 120); // 5 days (120h)
        assert_eq!(get_interval_for_score(6), 240); // 10 days (240h)
        assert_eq!(get_interval_for_score(7), 480); // 20 days (480h)
        assert_eq!(get_interval_for_score(8), 960); // 40 days (960h)
        assert_eq!(get_interval_for_score(9), 1440); // 60 days (1440h)
        assert_eq!(get_interval_for_score(10), 2160); // 90 days (2160h)
        assert_eq!(get_interval_for_score(100), 2160); // 90 days (2160h)
    }

    #[test]
    fn test_compute_next_review() {
        let now = Utc::now();

        // New card (0 correct, 0 wrong) should be 2 hours
        let next_review = compute_next_review(0, 0);
        let expected_hours = 2;
        let diff = (next_review - now).num_hours();
        assert_eq!(diff, expected_hours);

        // First correct (1 correct, 0 wrong) should be 4 hours
        let next_review = compute_next_review(1, 0);
        let expected_hours = 4;
        let diff = (next_review - now).num_hours();
        assert_eq!(diff, expected_hours);

        // Score 2 (2 correct, 0 wrong) should be 8 hours
        let next_review = compute_next_review(2, 0);
        let expected_hours = 8;
        let diff = (next_review - now).num_hours();
        assert_eq!(diff, expected_hours);

        // Score 3 (3 correct, 0 wrong) should be 1 day
        let next_review = compute_next_review(3, 0);
        let expected_days = 1;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);

        // Score 9 card (9 correct, 0 wrong) should be 60 days
        let next_review = compute_next_review(9, 0);
        let expected_days = 60;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);

        // Mastered card (10 correct, 0 wrong) should be 90 days
        let next_review = compute_next_review(10, 0);
        let expected_days = 90;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);

        // Struggling card (2 correct, 5 wrong) should be 2 hours
        let next_review = compute_next_review(2, 5);
        let expected_hours = 2;
        let diff = (next_review - now).num_hours();
        assert_eq!(diff, expected_hours);
    }
}
