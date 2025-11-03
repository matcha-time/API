use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Topic model - organizes decks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// Unique topic identifier
    pub id: i64,
    /// Topic name (max 255 chars for optimal indexing)
    pub name: String,
    /// Topic description (TEXT for longer content)
    pub description: Option<String>,
    /// Topic category (max 100 chars for optimal indexing)
    pub category: Option<String>,
    /// When the topic was created
    pub created_at: DateTime<Utc>,
    /// When the topic was last updated
    pub updated_at: DateTime<Utc>,
}

/// Deck model - contains vocabulary cards, organized by topics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    /// Unique deck identifier
    pub id: i64,
    /// Deck name (max 255 chars for optimal indexing)
    pub name: String,
    /// Deck description (TEXT for longer content)
    pub description: Option<String>,
    /// Topic ID this deck belongs to (indexed for fast lookups)
    pub topic_id: i64,
    /// When the deck was created
    pub created_at: DateTime<Utc>,
    /// When the deck was last updated
    pub updated_at: DateTime<Utc>,
}

/// Card model - vocabulary words in two languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    /// Unique card identifier
    pub id: i64,
    /// Deck ID this card belongs to (indexed for fast lookups)
    pub deck_id: i64,
    /// Topic ID this card belongs to (denormalized for direct topic access, indexed)
    pub topic_id: i64,
    /// Word in the first language (max 255 chars)
    pub word_lang1: String,
    /// Word in the second language (max 255 chars)
    pub word_lang2: String,
    /// Language code for the first language (ISO 639-1, 2 chars, indexed)
    pub lang1_code: String,
    /// Language code for the second language (ISO 639-1, 2 chars, indexed)
    pub lang2_code: String,
    /// Optional example sentence or usage (TEXT for longer content)
    pub example: Option<String>,
    /// When the card was created
    pub created_at: DateTime<Utc>,
    /// When the card was last updated
    pub updated_at: DateTime<Utc>,
}

/// User deck practice tracking - tracks user's overall practice on a deck
/// Optimized with unique constraint on (user_id, deck_id) for O(1) lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeckPractice {
    /// Unique identifier
    pub id: i64,
    /// User ID (indexed)
    pub user_id: i64,
    /// Deck ID (indexed, unique with user_id)
    pub deck_id: i64,
    /// User's practice score for this deck (double precision for decimal accuracy)
    pub score: f64,
    /// Number of times the user has practiced this deck (i32 sufficient for counts)
    pub practice_count: i32,
    /// Last time the user practiced this deck (nullable for new records)
    pub last_practiced_at: Option<DateTime<Utc>>,
    /// When this practice record was created
    pub created_at: DateTime<Utc>,
    /// When this practice record was last updated
    pub updated_at: DateTime<Utc>,
}

/// User card practice history - tracks each validation of a card
/// High-volume table optimized for fast inserts and time-based queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCardPractice {
    /// Unique identifier
    pub id: i64,
    /// User ID (indexed)
    pub user_id: i64,
    /// Card ID (indexed)
    pub card_id: i64,
    /// Whether the user successfully found the word in the other language
    pub validated: bool,
    /// When this validation occurred (indexed for time-based queries)
    pub validated_at: DateTime<Utc>,
    /// Direction of validation (e.g., "lang1_to_lang2" or "lang2_to_lang1")
    /// Fixed VARCHAR(20) for optimal storage
    pub direction: String,
}

/// Optimized insert struct for UserCardPractice
/// Used for batch inserts to minimize database round trips
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCardPracticeInsert {
    pub user_id: i64,
    pub card_id: i64,
    pub validated: bool,
    pub direction: String,
}

impl From<UserCardPracticeInsert> for UserCardPractice {
    fn from(insert: UserCardPracticeInsert) -> Self {
        Self {
            id: 0, // Will be set by database
            user_id: insert.user_id,
            card_id: insert.card_id,
            validated: insert.validated,
            validated_at: Utc::now(),
            direction: insert.direction,
        }
    }
}

/// Optimized query filters for common patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardQueryFilter {
    pub deck_id: Option<i64>,
    pub topic_id: Option<i64>,
    pub lang1_code: Option<String>,
    pub lang2_code: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCardPracticeQueryFilter {
    pub user_id: i64,
    pub card_id: Option<i64>,
    pub validated: Option<bool>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
