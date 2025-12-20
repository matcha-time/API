# mms-srs

Spaced Repetition System (SRS) library for Matcha Time.

This crate provides the core spaced repetition algorithm and related functionality for scheduling flashcard reviews.

## Features

- **Simple SRS Algorithm**: Uses a score-based approach (times_correct - times_wrong) to determine review intervals
- **Exponential Backoff**: Gradually increases intervals as cards are mastered
- **Well-tested**: Comprehensive unit tests for core functionality

## Algorithm

The SRS algorithm calculates a score as `times_correct - times_wrong` and applies the following intervals:

| Score | Interval | Description |
|-------|----------|-------------|
| ≤ 0   | 1 day    | New or struggling cards |
| 1     | 3 days   | First correct answer |
| 2     | 7 days   | 1 week |
| 3     | 14 days  | 2 weeks |
| 4     | 30 days  | 1 month |
| 5     | 60 days  | 2 months |
| 6     | 120 days | 4 months |
| ≥ 7   | 180 days | Well-mastered cards (6 months) |

## Usage

```rust
use mms_srs::compute_next_review;

// Calculate next review date for a card with 5 correct and 2 wrong answers
let next_review = compute_next_review(5, 2);
// Score = 3, so interval = 14 days
```

## API

### `compute_next_review(times_correct: i32, times_wrong: i32) -> DateTime<Utc>`

Computes the next review date based on the card's history.

### `calculate_score(times_correct: i32, times_wrong: i32) -> i32`

Calculates the current SRS score for a card.

### `get_interval_for_score(score: i32) -> i64`

Returns the interval in days for a given score.
