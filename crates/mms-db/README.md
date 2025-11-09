# mms-db

Database schema and models for the Matcha Time language learning application.

This crate contains the PostgreSQL database schema and related data structures for managing flashcards, decks, roadmaps, and user progress tracking.

## Database Schema

```mermaid
erDiagram
    %% User Group
    USERS {
        UUID id PK
        TEXT username
        TEXT email
        TEXT password_hash
        TIMESTAMPTZ created_at
    }
    USER_STATS {
        UUID user_id PK,FK
        INT current_streak_days
        INT longest_streak_days
        INT total_reviews
        INT total_cards_learned
        DATE last_review_date
        TIMESTAMPTZ created_at
        TIMESTAMPTZ updated_at
    }
    USER_ACTIVITY {
        UUID user_id PK,FK
        DATE activity_date PK
        INT reviews_count
    }
    USER_CARD_PROGRESS {
        UUID user_id PK,FK
        UUID flashcard_id PK,FK
        TIMESTAMPTZ next_review_at
        TIMESTAMPTZ last_review_at
        INT times_correct
        INT times_wrong
        TIMESTAMPTZ mastered_at
        TIMESTAMPTZ updated_at
    }
    USER_DECK_PROGRESS {
        UUID user_id PK,FK
        UUID deck_id PK,FK
        INT total_cards
        INT mastered_cards
        INT cards_due_today
        INT total_practices
        TIMESTAMPTZ last_practiced_at
        TIMESTAMPTZ updated_at
    }

    %% Flashcards Group
    FLASHCARDS {
        UUID id PK
        TEXT term
        TEXT translation
        CHAR language_from
        CHAR language_to
        TIMESTAMPTZ created_at
    }

    %% Decks Group
    DECKS {
        UUID id PK
        TEXT title
        TEXT description
        CHAR language_from
        CHAR language_to
        TIMESTAMPTZ created_at
    }
    DECK_FLASHCARDS {
        UUID deck_id PK,FK
        UUID flashcard_id PK,FK
    }

    %% Roadmaps Group
    ROADMAPS {
        UUID id PK
        TEXT title
        TEXT description
        CHAR language_from
        CHAR language_to
        TIMESTAMPTZ created_at
    }
    ROADMAP_NODES {
        UUID id PK
        UUID roadmap_id FK
        UUID deck_id FK
        UUID parent_node_id FK
        NUMERIC pos_x
        NUMERIC pos_y
        TIMESTAMPTZ created_at
    }

    ROADMAPS ||--o{ ROADMAP_NODES : "has"
    DECKS ||--o{ ROADMAP_NODES : "linked to"
    ROADMAP_NODES ||--o{ ROADMAP_NODES : "parent of"
    DECKS ||--|{ DECK_FLASHCARDS : "contains"
    FLASHCARDS ||--|{ DECK_FLASHCARDS : "in"
    USERS ||--o{ USER_CARD_PROGRESS : "tracks"
    FLASHCARDS ||--o{ USER_CARD_PROGRESS : "progress for"
    USERS ||--o{ USER_DECK_PROGRESS : "tracks"
    DECKS ||--o{ USER_DECK_PROGRESS : "progress for"
    USERS ||--|| USER_STATS : "has"
    USERS ||--o{ USER_ACTIVITY : "logs"
```

## Key Features

### Spaced Repetition System (SRS)

The database implements an SRS through the `user_card_progress` table, which tracks:
- Next review time for each flashcard (`next_review_at`)
- Review history (`times_correct`, `times_wrong`, `last_review_at`)
- Mastery status (`mastered_at`)

### Performance Optimizations

The schema includes several performance optimizations:

- **Partial indexes** for due cards (`next_review_at <= NOW()`) and mastered cards
- **Covering indexes** for practice session queries to avoid table lookups
- **Language pair indexes** for fast filtering of decks and roadmaps
- **Helper function** `refresh_deck_progress()` to efficiently update aggregated deck statistics

### Data Relationships

- **Flashcards** are reusable across multiple decks (many-to-many via `deck_flashcards`)
- **Roadmaps** organize decks into visual learning paths with positional coordinates
- **User Progress** is tracked at both card and deck levels for efficient queries
- **Activity Tracking** enables streak calculations and heatmap visualizations