//! SRS (Spaced Repetition System) library for Matcha Time
//!
//! This crate provides the core spaced repetition algorithm and related functionality
//! for scheduling flashcard reviews.

use chrono::{DateTime, Duration, Utc};

/// The score at which a card is considered mastered.
///
/// When `times_correct - times_wrong >= MASTERY_THRESHOLD`, the card reaches
/// its maximum review interval and is flagged as mastered.
pub const MASTERY_THRESHOLD: i32 = 10;

/// SRS intervals in hours, indexed by score.
///
/// Scores 0-2 use hour-based intervals for aggressive early practice,
/// then transition to day-based intervals with exponential growth.
///
/// | Index | Score | Interval        |
/// |-------|-------|-----------------|
/// | 0     | ≤ 0   | 2 hours         |
/// | 1     | 1     | 4 hours         |
/// | 2     | 2     | 8 hours         |
/// | 3     | 3     | 1 day (24h)     |
/// | 4     | 4     | 2 days (48h)    |
/// | 5     | 5     | 5 days (120h)   |
/// | 6     | 6     | 10 days (240h)  |
/// | 7     | 7     | 20 days (480h)  |
/// | 8     | 8     | 40 days (960h)  |
/// | 9     | 9     | 60 days (1440h) |
/// | 10    | ≥ 10  | 90 days (2160h) |
const INTERVALS_HOURS: [i64; 11] = [
    2,    // score ≤ 0: 2 hours - immediate retry
    4,    // score 1: 4 hours
    8,    // score 2: 8 hours
    24,   // score 3: 1 day
    48,   // score 4: 2 days
    120,  // score 5: 5 days
    240,  // score 6: 10 days
    480,  // score 7: 20 days (~3 weeks)
    960,  // score 8: 40 days (~6 weeks)
    1440, // score 9: 60 days (2 months)
    2160, // score ≥ 10: 90 days (3 months, mastered)
];

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
/// * `now` - The current time, for deterministic scheduling
///
/// # Returns
///
/// The next review date as a `DateTime<Utc>`
pub fn compute_next_review(
    times_correct: i32,
    times_wrong: i32,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    let hours = get_interval_for_score(calculate_score(times_correct, times_wrong));
    now + Duration::hours(hours)
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

/// Returns true if the given score means the card is mastered.
pub fn is_mastered(times_correct: i32, times_wrong: i32) -> bool {
    calculate_score(times_correct, times_wrong) >= MASTERY_THRESHOLD
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
pub fn get_interval_for_score(score: i32) -> i64 {
    let index = score.clamp(0, INTERVALS_HOURS.len() as i32 - 1) as usize;
    INTERVALS_HOURS[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap()
    }

    #[test]
    fn test_calculate_score() {
        assert_eq!(calculate_score(5, 2), 3);
        assert_eq!(calculate_score(0, 0), 0);
        assert_eq!(calculate_score(2, 5), -3);
    }

    #[test]
    fn test_is_mastered() {
        assert!(!is_mastered(0, 0));
        assert!(!is_mastered(9, 0));
        assert!(is_mastered(10, 0));
        assert!(is_mastered(15, 3));
        assert!(!is_mastered(10, 1));
    }

    #[test]
    fn test_mastery_threshold_matches_interval_table() {
        // The mastery threshold should correspond to the last index in the interval table
        assert_eq!(MASTERY_THRESHOLD as usize, INTERVALS_HOURS.len() - 1);
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
        assert_eq!(get_interval_for_score(100), 2160); // clamped to max
    }

    #[test]
    fn test_compute_next_review_deterministic() {
        let now = fixed_now();

        // New card (0 correct, 0 wrong) should be 2 hours from now
        let next = compute_next_review(0, 0, now);
        assert_eq!((next - now).num_hours(), 2);

        // First correct (1 correct, 0 wrong) should be 4 hours
        let next = compute_next_review(1, 0, now);
        assert_eq!((next - now).num_hours(), 4);

        // Score 2 should be 8 hours
        let next = compute_next_review(2, 0, now);
        assert_eq!((next - now).num_hours(), 8);

        // Score 3 should be 1 day
        let next = compute_next_review(3, 0, now);
        assert_eq!((next - now).num_days(), 1);

        // Score 9 should be 60 days
        let next = compute_next_review(9, 0, now);
        assert_eq!((next - now).num_days(), 60);

        // Mastered card (10 correct, 0 wrong) should be 90 days
        let next = compute_next_review(10, 0, now);
        assert_eq!((next - now).num_days(), 90);

        // Struggling card (2 correct, 5 wrong) should be 2 hours
        let next = compute_next_review(2, 5, now);
        assert_eq!((next - now).num_hours(), 2);
    }

    #[test]
    fn test_compute_next_review_exact_timestamp() {
        let now = fixed_now();

        // Verify exact timestamp, not just hour/day rounding
        let next = compute_next_review(0, 0, now);
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 6, 15, 14, 0, 0).unwrap());

        let next = compute_next_review(3, 0, now);
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap());
    }
}
