use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub native_language: Option<String>,
    pub learning_language: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Roadmap {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub language_from: String,
    pub language_to: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Deck {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub language_from: String,
    pub language_to: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Flashcard {
    pub id: Uuid,
    pub term: String,
    pub translation: String,
    pub language_from: String,
    pub language_to: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RoadmapNodeWithProgress {
    pub node_id: Uuid,
    pub parent_node_id: Option<Uuid>,
    pub pos_x: i32,
    pub pos_y: i32,
    pub deck_id: Uuid,
    pub deck_title: String,
    pub deck_description: Option<String>,
    pub total_cards: i32,
    pub mastered_cards: i32,
    pub cards_due_today: i32,
    pub total_practices: i32,
    pub last_practiced_at: Option<DateTime<Utc>>,
    pub progress_percentage: f64,
    pub next_practice_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct RoadmapWithProgress {
    pub roadmap: RoadmapMetadata,
    pub nodes: Vec<RoadmapNodeWithProgress>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FlashcardWithProgress {
    pub id: Uuid,
    pub term: String,
    pub translation: String,
    pub next_review_at: Option<DateTime<Utc>>,
    pub last_review_at: Option<DateTime<Utc>>,
    pub times_correct: i32,
    pub times_wrong: i32,
    pub mastered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserStats {
    pub current_streak_days: i32,
    pub longest_streak_days: i32,
    pub total_reviews: i32,
    pub total_cards_learned: i32,
    pub last_review_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ActivityDay {
    pub activity_date: NaiveDate,
    pub reviews_count: i32,
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
