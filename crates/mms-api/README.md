# API Endpoints

## Health

- `GET /health` - Health check
  - **Response:** `200 OK`

## Authentication

- `GET /auth/google` - Initiate Google OAuth flow
  - **Response:** Redirect to Google OAuth

- `GET /auth/callback` - OAuth callback handler
  - **Response:** `200 OK`
  ```json
  {
    "token": "jwt_token_string",
    "user": {}
  }
  ```

## Roadmaps

- `GET /roadmaps` - List all roadmaps
  - **Response:** `200 OK`
  ```json
  [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Spanish to English Learning Path",
      "description": "A comprehensive roadmap",
      "language_from": "es",
      "language_to": "en"
    },
    // ...
  ]
  ```

- `GET /roadmaps/{language_from}/{language_to}` - Get roadmaps by language pair
  - **Response:** `200 OK` (same as above)

- `GET /roadmaps/{roadmap_id}/progress/{user_id}` - Get roadmap with user progress
  - **Response:** `200 OK`
  ```json
  [
    {
      "node_id": "770e8400-e29b-41d4-a716-446655440000",
      "pos_x": 100,
      "pos_y": 50,
      "deck_id": "880e8400-e29b-41d4-a716-446655440000",
      "deck_title": "Basic Greetings",
      "total_cards": 20,
      "mastered_cards": 15,
      "cards_due_today": 3,
      "total_practices": 45,
      "last_practiced_at": "2024-01-15T10:30:00Z"
    },
    // ...
  ]
  ```

## Users

- `GET /users/{user_id}/dashboard` - Get user dashboard stats and activity
  - **Response:** `200 OK`
  ```json
  {
    "stats": {
      "current_streak_days": 5,
      "longest_streak_days": 10,
      "total_reviews": 150,
      "total_cards_learned": 50,
      "last_review_date": "2024-01-15"
    },
    "heatmap": [
      {
        "activity_date": "2024-01-15",
        "reviews_count": 10
      },
      // ...
    ]
  }
  ```

## Decks

- `GET /decks/{deck_id}/practice/{user_id}` - Get practice session cards for a deck
  - **Response:** `200 OK`
  ```json
  [
    {
      "id": "990e8400-e29b-41d4-a716-446655440000",
      "term": "Hola",
      "translation": "Hello",
      "times_correct": 5,
      "times_wrong": 2
    },
    // ...
  ]
  ```

## Practice

- `POST /practice/{user_id}/{flashcard_id}/review` - Submit a flashcard review
  - **Request Body:**
  ```json
  {
    "correct": true,
    "next_review_at": "2024-01-15T10:30:00Z",
    "deck_id": "880e8400-e29b-41d4-a716-446655440000"
  }
  ```
  - **Response:** `200 OK`

