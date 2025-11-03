# Database Model

## Performance Optimizations

This schema is optimized for **blazing fast insertions and reads** with:
- Strategic indexes on foreign keys and frequently queried fields
- Composite indexes for common query patterns
- Partial indexes for time-based queries
- Unique constraints for O(1) lookups
- Optimized data types (VARCHAR vs TEXT, CHAR for codes)
- Denormalization where it improves query performance

See [OPTIMIZATION.md](./OPTIMIZATION.md) for detailed performance strategies.

## Entity Relationship Diagram

```mermaid
erDiagram
    User ||--o{ UserDeckPractice : "has"
    User ||--o{ UserCardPractice : "has"
    
    Topic ||--o{ Deck : "contains"
    Topic ||--o{ Card : "contains"
    
    Deck ||--o{ Card : "has"
    Deck ||--o{ UserDeckPractice : "tracked by"
    
    Card ||--o{ UserCardPractice : "practiced in"
    
    User {
        i64 id PK
        string username
        string email
        string name
    }
    
    Topic {
        i64 id PK
        string name
        string description
        string category
        datetime created_at
        datetime updated_at
    }
    
    Deck {
        i64 id PK
        i64 topic_id FK
        string name
        string description
        datetime created_at
        datetime updated_at
    }
    
    Card {
        i64 id PK
        i64 deck_id FK
        i64 topic_id FK
        string word_lang1
        string word_lang2
        string lang1_code
        string lang2_code
        string example
        datetime created_at
        datetime updated_at
    }
    
    UserDeckPractice {
        i64 id PK
        i64 user_id FK
        i64 deck_id FK
        float score
        int practice_count
        datetime last_practiced_at
        datetime created_at
        datetime updated_at
    }
    
    UserCardPractice {
        i64 id PK
        i64 user_id FK
        i64 card_id FK
        bool validated
        datetime validated_at
        string direction
    }
```

## Relationships

- **Topic → Deck**: One-to-Many (one topic can have many decks)
- **Topic → Card**: One-to-Many (one topic can have many cards)
- **Deck → Card**: One-to-Many (one deck contains many cards)
- **User → UserDeckPractice**: One-to-Many (user can practice multiple decks)
- **User → UserCardPractice**: One-to-Many (user can practice multiple cards)
- **Deck → UserDeckPractice**: One-to-Many (deck can be practiced by multiple users)
- **Card → UserCardPractice**: One-to-Many (card can be practiced multiple times by users)

