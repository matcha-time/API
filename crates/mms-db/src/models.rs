use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    id: Uuid,
    username: String,
    email: String,
    native_language: Option<String>,
    learning_language: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Roadmap {
    id: Uuid,
    title: String,
    description: Option<String>,
    language_from: String,
    language_to: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Deck {
    id: Uuid,
    title: String,
    description: Option<String>,
    language_from: String,
    language_to: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Flashcard {
    id: Uuid,
    term: String,
    translation: String,
    language_from: String,
    language_to: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RoadmapNodeWithProgress {
    node_id: Uuid,
    parent_node_id: Option<Uuid>,
    pos_x: i32,
    pos_y: i32,
    deck_id: Uuid,
    deck_title: String,
    deck_description: Option<String>,
    total_cards: i32,
    mastered_cards: i32,
    cards_due_today: i32,
    total_practices: i32,
    last_practiced_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct RoadmapWithProgress {
    pub roadmap: RoadmapMetadata,
    pub nodes: Vec<RoadmapNodeWithProgress>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RoadmapMetadata {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub language_from: String,
    pub language_to: String,
    pub total_nodes: i32,
    pub completed_nodes: i32,
    pub progress_percentage: f64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FlashcardWithProgress {
    id: Uuid,
    term: String,
    translation: String,
    next_review_at: Option<DateTime<Utc>>,
    last_review_at: Option<DateTime<Utc>>,
    times_correct: i32,
    times_wrong: i32,
    mastered_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct UserStats {
    current_streak_days: i32,
    longest_streak_days: i32,
    total_reviews: i32,
    total_cards_learned: i32,
    last_review_date: Option<NaiveDate>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ActivityDay {
    activity_date: NaiveDate,
    reviews_count: i32,
}
