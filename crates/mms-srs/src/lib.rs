//! SRS (Spaced Repetition System) library for Matcha Time
//!
//! This crate provides the core spaced repetition algorithm and related functionality
//! for scheduling flashcard reviews.

use chrono::{DateTime, Duration, Utc};

/// Compute the next review date based on the SRS algorithm.
///
/// Uses a simple exponential backoff based on the score (times_correct - times_wrong).
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
/// the following intervals:
///
/// * Score ≤ 0: 1 day (new or struggling cards)
/// * Score 1: 3 days (first correct answer)
/// * Score 2: 7 days (1 week)
/// * Score 3: 14 days (2 weeks)
/// * Score 4: 30 days (1 month)
/// * Score 5: 60 days (2 months)
/// * Score 6: 120 days (4 months)
/// * Score ≥ 7: 180 days (6 months, well-mastered cards)
pub fn compute_next_review(times_correct: i32, times_wrong: i32) -> DateTime<Utc> {
    let score = times_correct - times_wrong;

    // SRS intervals in days based on score
    let interval_days = match score {
        s if s <= 0 => 1, // 1 day for new or struggling cards
        1 => 3,           // 3 days after first correct answer
        2 => 7,           // 1 week
        3 => 14,          // 2 weeks
        4 => 30,          // 1 month
        5 => 60,          // 2 months
        6 => 120,         // 4 months
        _ => 180,         // 6 months for well-mastered cards
    };

    Utc::now() + Duration::days(interval_days)
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

/// Get the interval in days for a given score.
///
/// # Arguments
///
/// * `score` - The SRS score (typically times_correct - times_wrong)
///
/// # Returns
///
/// The interval in days as an `i64`
pub fn get_interval_for_score(score: i32) -> i64 {
    match score {
        s if s <= 0 => 1,
        1 => 3,
        2 => 7,
        3 => 14,
        4 => 30,
        5 => 60,
        6 => 120,
        _ => 180,
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
        assert_eq!(get_interval_for_score(-1), 1);
        assert_eq!(get_interval_for_score(0), 1);
        assert_eq!(get_interval_for_score(1), 3);
        assert_eq!(get_interval_for_score(2), 7);
        assert_eq!(get_interval_for_score(3), 14);
        assert_eq!(get_interval_for_score(4), 30);
        assert_eq!(get_interval_for_score(5), 60);
        assert_eq!(get_interval_for_score(6), 120);
        assert_eq!(get_interval_for_score(7), 180);
        assert_eq!(get_interval_for_score(100), 180);
    }

    #[test]
    fn test_compute_next_review() {
        let now = Utc::now();

        // New card (0 correct, 0 wrong) should be 1 day
        let next_review = compute_next_review(0, 0);
        let expected_days = 1;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);

        // First correct (1 correct, 0 wrong) should be 3 days
        let next_review = compute_next_review(1, 0);
        let expected_days = 3;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);

        // Well-mastered card (10 correct, 0 wrong) should be 180 days
        let next_review = compute_next_review(10, 0);
        let expected_days = 180;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);

        // Struggling card (2 correct, 5 wrong) should be 1 day
        let next_review = compute_next_review(2, 5);
        let expected_days = 1;
        let diff = (next_review - now).num_days();
        assert_eq!(diff, expected_days);
    }
}
