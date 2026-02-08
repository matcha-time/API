# mms-srs

Spaced Repetition System (SRS) library for Matcha Time.

This crate provides the core spaced repetition algorithm and related functionality for scheduling flashcard reviews.

## Features

- **Simple SRS Algorithm**: Uses a score-based approach (times_correct - times_wrong) to determine review intervals
- **Aggressive Early Practice**: Hour-based intervals for new cards, transitioning to days with exponential growth
- **Mastery Tracking**: Cards reaching score >= 10 are considered mastered (90-day review cycle)
- **Deterministic Scheduling**: `compute_next_review` accepts a `now` parameter for testability
- **Well-tested**: Comprehensive unit tests including exact timestamp verification

## Algorithm

The SRS algorithm calculates a score as `times_correct - times_wrong` and maps it to an interval from the `INTERVALS_HOURS` table:

| Score | Interval | Description |
| ----- | -------- | ----------- |
| <= 0 | 2 hours | New or struggling cards |
| 1 | 4 hours | First correct answer |
| 2 | 8 hours | Building confidence |
| 3 | 1 day | Transitioning to daily |
| 4 | 2 days | Short-term retention |
| 5 | 5 days | Medium-term retention |
| 6 | 10 days | ~1.5 weeks |
| 7 | 20 days | ~3 weeks |
| 8 | 40 days | ~6 weeks |
| 9 | 60 days | 2 months |
| >= 10 | 90 days | Mastered (3 months) |

## Constants

- **`MASTERY_THRESHOLD`** (`10`): The score at which a card is considered mastered. This constant is the single source of truth, shared with the database layer via the `refresh_deck_progress` SQL function parameter.

## Usage

```rust
use mms_srs::{compute_next_review, is_mastered, MASTERY_THRESHOLD};
use chrono::Utc;

let now = Utc::now();

// Calculate next review date for a card with 5 correct and 2 wrong answers
let next_review = compute_next_review(5, 2, now);
// Score = 3, so interval = 1 day from now

// Check if a card is mastered
assert!(!is_mastered(5, 2));   // score 3 < 10
assert!(is_mastered(12, 1));   // score 11 >= 10
```

## API

### `compute_next_review(times_correct: i32, times_wrong: i32, now: DateTime<Utc>) -> DateTime<Utc>`

Computes the next review date based on the card's history. Accepts `now` for deterministic, testable scheduling.

### `calculate_score(times_correct: i32, times_wrong: i32) -> i32`

Calculates the current SRS score for a card (`times_correct - times_wrong`).

### `is_mastered(times_correct: i32, times_wrong: i32) -> bool`

Returns `true` if the card's score meets or exceeds `MASTERY_THRESHOLD`.

### `get_interval_for_score(score: i32) -> i64`

Returns the interval in **hours** for a given score. Scores are clamped to the valid range.
