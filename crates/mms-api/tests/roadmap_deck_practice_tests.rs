use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create test roadmap and deck data
async fn create_test_roadmap_and_decks(pool: &PgPool) -> anyhow::Result<(Uuid, Uuid, Uuid)> {
    // Create a roadmap with unique ID in title to avoid conflicts
    let roadmap_id = Uuid::new_v4();
    let unique_title = format!("Test Spanish Roadmap {}", roadmap_id);
    sqlx::query(
        r#"
        INSERT INTO roadmaps (id, title, description, language_from, language_to, created_at)
        VALUES ($1, $2, 'Learn Spanish from English', 'en', 'es', NOW())
        "#,
    )
    .bind(roadmap_id)
    .bind(&unique_title)
    .execute(pool)
    .await?;

    // Create deck 1 (Basics)
    let deck1_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO decks (id, title, description, language_from, language_to, created_at)
        VALUES ($1, 'Spanish Basics', 'Basic Spanish vocabulary', 'en', 'es', NOW())
        "#,
    )
    .bind(deck1_id)
    .execute(pool)
    .await?;

    // Create deck 2 (Advanced)
    let deck2_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO decks (id, title, description, language_from, language_to, created_at)
        VALUES ($1, 'Spanish Advanced', 'Advanced Spanish vocabulary', 'en', 'es', NOW())
        "#,
    )
    .bind(deck2_id)
    .execute(pool)
    .await?;

    // Link decks to roadmap nodes
    sqlx::query(
        r#"
        INSERT INTO roadmap_nodes (roadmap_id, deck_id, pos_x, pos_y, created_at)
        VALUES ($1, $2, 0, 0, NOW()), ($1, $3, 1, 0, NOW())
        "#,
    )
    .bind(roadmap_id)
    .bind(deck1_id)
    .bind(deck2_id)
    .execute(pool)
    .await?;

    // Create flashcards for deck 1 with unique IDs in content to avoid duplicates
    let flashcard1_id = Uuid::new_v4();
    let flashcard2_id = Uuid::new_v4();
    let unique_suffix = format!("_{}", Uuid::new_v4().to_string()[..8].to_string());

    sqlx::query(
        r#"
        INSERT INTO flashcards (id, term, translation, language_from, language_to, created_at)
        VALUES
            ($1, $3, 'hola', 'en', 'es', NOW()),
            ($2, $4, 'adiós', 'en', 'es', NOW())
        "#,
    )
    .bind(flashcard1_id)
    .bind(flashcard2_id)
    .bind(format!("hello{}", unique_suffix))
    .bind(format!("goodbye{}", unique_suffix))
    .execute(pool)
    .await?;

    // Link flashcards to deck
    sqlx::query(
        r#"
        INSERT INTO deck_flashcards (deck_id, flashcard_id)
        VALUES ($1, $2), ($1, $3)
        "#,
    )
    .bind(deck1_id)
    .bind(flashcard1_id)
    .bind(flashcard2_id)
    .execute(pool)
    .await?;

    Ok((roadmap_id, deck1_id, deck2_id))
}

#[tokio::test]
async fn test_get_all_roadmaps() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create test data
    let (roadmap_id, _, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Get all roadmaps
    let response = client.get("/roadmaps").await;
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json.is_array(), "Response should be an array");

    let roadmaps = json.as_array().unwrap();
    assert!(!roadmaps.is_empty(), "Should have at least one roadmap");

    // Find our test roadmap
    let test_roadmap = roadmaps
        .iter()
        .find(|r| r["id"].as_str().unwrap() == roadmap_id.to_string())
        .expect("Test roadmap should be in response");

    assert!(
        test_roadmap["title"].as_str().unwrap().starts_with("Test Spanish Roadmap"),
        "Roadmap title should start with 'Test Spanish Roadmap'"
    );
    assert_eq!(test_roadmap["language_from"].as_str().unwrap(), "en");
    assert_eq!(test_roadmap["language_to"].as_str().unwrap(), "es");

    // Cleanup - delete only this test's roadmap (cascades to decks, flashcards, etc.)
    common::db::delete_roadmap_by_id(&state.pool, roadmap_id)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_get_roadmaps_by_language_pair() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create test data
    create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Filter by language pair
    let response = client.get("/roadmaps/en/es").await;
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    let roadmaps = json.as_array().unwrap();

    assert!(!roadmaps.is_empty(), "Should have Spanish roadmap");
    assert!(roadmaps
        .iter()
        .all(|r| r["language_from"].as_str().unwrap() == "en"
            && r["language_to"].as_str().unwrap() == "es"));

    // Try non-existent language pair (fr/es - valid codes but no data)
    let response = client.get("/roadmaps/fr/es").await;
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    let roadmaps = json.as_array().unwrap();
    assert!(roadmaps.is_empty(), "Should have no French-Spanish roadmaps");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_get_roadmap_with_progress_authenticated() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create user
    let user_id = common::db::create_verified_user(&state.pool, "roadmap@example.com", "roadmapuser")
        .await
        .expect("Failed to create user");

    // Create test data
    let (roadmap_id, deck1_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    // Create some progress for the user
    sqlx::query(
        r#"
        INSERT INTO user_deck_progress (user_id, deck_id, total_cards, mastered_cards, cards_due_today)
        VALUES ($1, $2, 2, 1, 1)
        "#,
    )
    .bind(user_id)
    .bind(deck1_id)
    .execute(&state.pool)
    .await
    .expect("Failed to create progress");

    let token = common::jwt::create_test_token(user_id, "roadmap@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Get roadmap with progress
    let response = client
        .get_with_auth(
            &format!("/roadmaps/{}/progress/{}", roadmap_id, user_id),
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json.is_array(), "Response should be array of nodes");

    let nodes = json.as_array().unwrap();
    assert!(!nodes.is_empty(), "Should have roadmap nodes");

    // Find node with progress
    let node_with_progress = nodes
        .iter()
        .find(|n| n["deck_id"].as_str().unwrap() == deck1_id.to_string())
        .expect("Should find deck node");

    assert_eq!(
        node_with_progress["total_cards"].as_i64().unwrap(),
        2,
        "Should have 2 total cards"
    );
    assert_eq!(
        node_with_progress["mastered_cards"].as_i64().unwrap(),
        1,
        "Should have 1 mastered card"
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_get_roadmap_progress_unauthorized() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create two users
    let user1_id = common::db::create_verified_user(&state.pool, "user1@example.com", "user1")
        .await
        .expect("Failed to create user1");

    let user2_id = common::db::create_verified_user(&state.pool, "user2@example.com", "user2")
        .await
        .expect("Failed to create user2");

    // Create roadmap
    let (roadmap_id, _, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    // User1 tries to access user2's progress
    let token = common::jwt::create_test_token(user1_id, "user1@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client
        .get_with_auth(
            &format!("/roadmaps/{}/progress/{}", roadmap_id, user2_id),
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_get_practice_session_for_deck() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create user
    let user_id = common::db::create_verified_user(&state.pool, "practice@example.com", "practiceuser")
        .await
        .expect("Failed to create user");

    // Create test data
    let (_, deck_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    let token = common::jwt::create_test_token(user_id, "practice@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Get practice session (new cards)
    let response = client
        .get_with_auth(
            &format!("/decks/{}/practice/{}", deck_id, user_id),
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json.is_array(), "Response should be array of flashcards");

    let cards = json.as_array().unwrap();
    assert_eq!(cards.len(), 2, "Should have 2 flashcards due for review");

    // Verify card structure
    let card = &cards[0];
    assert!(card["id"].is_string());
    assert!(card["term"].is_string());
    assert!(card["translation"].is_string());
    assert_eq!(card["times_correct"].as_i64().unwrap_or(0), 0);
    assert_eq!(card["times_wrong"].as_i64().unwrap_or(0), 0);

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_get_practice_session_unauthorized() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create two users
    let user1_id = common::db::create_verified_user(&state.pool, "user1practice@example.com", "user1")
        .await
        .expect("Failed to create user1");

    let user2_id = common::db::create_verified_user(&state.pool, "user2practice@example.com", "user2")
        .await
        .expect("Failed to create user2");

    // Create deck
    let (_, deck_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    // User1 tries to get user2's practice session
    let token = common::jwt::create_test_token(user1_id, "user1practice@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client
        .get_with_auth(
            &format!("/decks/{}/practice/{}", deck_id, user2_id),
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_submit_review_correct_answer() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create user
    let user_id = common::db::create_verified_user(&state.pool, "review@example.com", "reviewuser")
        .await
        .expect("Failed to create user");

    // Create test data
    let (_, deck_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    // Get a flashcard ID
    let flashcard_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM flashcards WHERE language_from = 'en' AND language_to = 'es' LIMIT 1",
    )
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get flashcard");

    let token = common::jwt::create_test_token(user_id, "review@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Submit correct review
    let review_body = json!({
        "correct": true,
        "next_review_at": "2025-12-01T10:00:00Z",
        "deck_id": deck_id.to_string()
    });

    let response = client
        .post_json_with_auth(
            &format!("/practice/{}/{}/review", user_id, flashcard_id),
            &review_body,
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::OK);

    // Verify progress was recorded
    let times_correct: i32 = sqlx::query_scalar(
        r#"
        SELECT times_correct
        FROM user_card_progress
        WHERE user_id = $1 AND flashcard_id = $2
        "#,
    )
    .bind(user_id)
    .bind(flashcard_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get progress");

    assert_eq!(times_correct, 1, "Should have 1 correct answer");

    // Verify deck progress was updated
    let deck_progress_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_deck_progress WHERE user_id = $1 AND deck_id = $2)",
    )
    .bind(user_id)
    .bind(deck_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to check deck progress");

    assert!(deck_progress_exists, "Deck progress should be created");

    // Verify activity was recorded
    let activity_count: i32 = sqlx::query_scalar(
        "SELECT reviews_count FROM user_activity WHERE user_id = $1 AND activity_date = CURRENT_DATE",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .expect("Failed to get activity")
    .unwrap_or(0);

    assert!(activity_count > 0, "Activity should be recorded");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_submit_review_wrong_answer() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create user
    let user_id = common::db::create_verified_user(&state.pool, "wrong@example.com", "wronguser")
        .await
        .expect("Failed to create user");

    // Create test data
    let (_, deck_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    // Get a flashcard ID
    let flashcard_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM flashcards WHERE language_from = 'en' AND language_to = 'es' LIMIT 1",
    )
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get flashcard");

    let token = common::jwt::create_test_token(user_id, "wrong@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Submit wrong review
    let review_body = json!({
        "correct": false,
        "next_review_at": "2025-11-27T12:00:00Z",
        "deck_id": deck_id.to_string()
    });

    let response = client
        .post_json_with_auth(
            &format!("/practice/{}/{}/review", user_id, flashcard_id),
            &review_body,
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::OK);

    // Verify progress was recorded
    let times_wrong: i32 = sqlx::query_scalar(
        r#"
        SELECT times_wrong
        FROM user_card_progress
        WHERE user_id = $1 AND flashcard_id = $2
        "#,
    )
    .bind(user_id)
    .bind(flashcard_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get progress");

    assert_eq!(times_wrong, 1, "Should have 1 wrong answer");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_submit_review_updates_stats() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create user
    let user_id = common::db::create_verified_user(&state.pool, "stats@example.com", "statsuser")
        .await
        .expect("Failed to create user");

    // Create test data
    let (_, deck_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    let flashcard_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM flashcards WHERE language_from = 'en' AND language_to = 'es' LIMIT 1",
    )
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get flashcard");

    // Get initial stats
    let initial_reviews: i32 = sqlx::query_scalar(
        "SELECT total_reviews FROM user_stats WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get initial stats");

    let token = common::jwt::create_test_token(user_id, "stats@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Submit review
    let review_body = json!({
        "correct": true,
        "next_review_at": "2025-12-01T10:00:00Z",
        "deck_id": deck_id.to_string()
    });

    client
        .post_json_with_auth(
            &format!("/practice/{}/{}/review", user_id, flashcard_id),
            &review_body,
            &token,
            &state.cookie_key,
        )
        .await;

    // Get updated stats
    let updated_reviews: i32 = sqlx::query_scalar(
        "SELECT total_reviews FROM user_stats WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get updated stats");

    assert_eq!(
        updated_reviews,
        initial_reviews + 1,
        "Total reviews should increase by 1"
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_submit_review_unauthorized() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create two users
    let user1_id = common::db::create_verified_user(&state.pool, "user1review@example.com", "user1")
        .await
        .expect("Failed to create user1");

    let user2_id = common::db::create_verified_user(&state.pool, "user2review@example.com", "user2")
        .await
        .expect("Failed to create user2");

    // Create deck
    let (_, deck_id, _) = create_test_roadmap_and_decks(&state.pool)
        .await
        .expect("Failed to create test data");

    let flashcard_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM flashcards WHERE language_from = 'en' AND language_to = 'es' LIMIT 1",
    )
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get flashcard");

    // User1 tries to submit review for user2
    let token = common::jwt::create_test_token(user1_id, "user1review@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let review_body = json!({
        "correct": true,
        "next_review_at": "2025-12-01T10:00:00Z",
        "deck_id": deck_id.to_string()
    });

    let response = client
        .post_json_with_auth(
            &format!("/practice/{}/{}/review", user2_id, flashcard_id),
            &review_body,
            &token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}
