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
    progress_percentage: f64,
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

// --- Query-specific structs (replacing tuple queries) ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub profile_picture_url: Option<String>,
    pub native_language: Option<String>,
    pub learning_language: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserWithGoogleId {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub google_id: Option<String>,
    pub profile_picture_url: Option<String>,
    pub native_language: Option<String>,
    pub learning_language: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserCredentials {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub profile_picture_url: Option<String>,
    pub email_verified: bool,
    pub native_language: Option<String>,
    pub learning_language: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserPasswordInfo {
    pub email: String,
    pub username: String,
    pub password_hash: Option<String>,
    pub auth_provider: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserExistenceCheck {
    pub id: Uuid,
    pub email_verified: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserVerificationInfo {
    pub id: Uuid,
    pub username: String,
    pub email_verified: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserIdAndName {
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserEmailAndName {
    pub email: String,
    pub username: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EmailVerifiedStatus {
    pub email: String,
    pub email_verified: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RefreshTokenRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CardProgress {
    pub next_review_at: DateTime<Utc>,
    pub times_correct: i32,
    pub times_wrong: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PracticeCard {
    pub id: Uuid,
    pub term: String,
    pub translation: String,
    pub times_correct: i32,
    pub times_wrong: i32,
}
