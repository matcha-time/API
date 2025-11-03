/// Database schema definitions with optimized indexes for high-performance operations
///
/// This schema is designed for:
/// - Fast inserts (minimal indexes that don't slow down writes)
/// - Fast reads (strategic indexes on frequently queried columns)
/// - Scalability (composite indexes for common query patterns)
/// - Data integrity (foreign keys and constraints)

/// SQL schema for topics table
pub const TOPICS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS topics (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for topics
CREATE INDEX IF NOT EXISTS idx_topics_category ON topics(category) WHERE category IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_topics_created_at ON topics(created_at DESC);
"#;

/// SQL schema for decks table
pub const DECKS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS decks (
    id BIGSERIAL PRIMARY KEY,
    topic_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_decks_topic FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
);

-- Critical indexes for decks (topic_id is frequently queried)
CREATE INDEX IF NOT EXISTS idx_decks_topic_id ON decks(topic_id);
CREATE INDEX IF NOT EXISTS idx_decks_created_at ON decks(created_at DESC);

-- Composite index for common query: get decks by topic ordered by creation
CREATE INDEX IF NOT EXISTS idx_decks_topic_created ON decks(topic_id, created_at DESC);
"#;

/// SQL schema for cards table
pub const CARDS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS cards (
    id BIGSERIAL PRIMARY KEY,
    deck_id BIGINT NOT NULL,
    topic_id BIGINT NOT NULL,
    word_lang1 VARCHAR(255) NOT NULL,
    word_lang2 VARCHAR(255) NOT NULL,
    lang1_code CHAR(2) NOT NULL,
    lang2_code CHAR(2) NOT NULL,
    example TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_cards_deck FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
    CONSTRAINT fk_cards_topic FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
);

-- Critical indexes for cards
CREATE INDEX IF NOT EXISTS idx_cards_deck_id ON cards(deck_id);
CREATE INDEX IF NOT EXISTS idx_cards_topic_id ON cards(topic_id);

-- Composite index for common query: get cards by deck (most frequent)
CREATE INDEX IF NOT EXISTS idx_cards_deck_created ON cards(deck_id, created_at DESC);

-- Composite index for querying by topic (for topic-based card access)
CREATE INDEX IF NOT EXISTS idx_cards_topic_created ON cards(topic_id, created_at DESC);

-- Index for language-based queries (if needed for filtering)
CREATE INDEX IF NOT EXISTS idx_cards_languages ON cards(lang1_code, lang2_code);
"#;

/// SQL schema for user_deck_practice table
/// This table tracks user's overall practice scores per deck
pub const USER_DECK_PRACTICE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS user_deck_practice (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    deck_id BIGINT NOT NULL,
    score DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    practice_count INTEGER NOT NULL DEFAULT 0,
    last_practiced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_udp_deck FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
    CONSTRAINT uq_user_deck UNIQUE (user_id, deck_id)
);

-- Critical indexes for user_deck_practice
-- Unique constraint above already creates index on (user_id, deck_id)
CREATE INDEX IF NOT EXISTS idx_udp_user_id ON user_deck_practice(user_id);
CREATE INDEX IF NOT EXISTS idx_udp_deck_id ON user_deck_practice(deck_id);

-- Composite index for common query: get all decks for a user ordered by last practice
CREATE INDEX IF NOT EXISTS idx_udp_user_last_practiced ON user_deck_practice(user_id, last_practiced_at DESC NULLS LAST);

-- Composite index for common query: get user's deck scores
CREATE INDEX IF NOT EXISTS idx_udp_user_score ON user_deck_practice(user_id, score DESC);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_udp_last_practiced ON user_deck_practice(last_practiced_at DESC) WHERE last_practiced_at IS NOT NULL;
"#;

/// SQL schema for user_card_practice table
/// This table tracks each individual card validation (high-volume writes)
/// Optimized for fast inserts and time-based queries
pub const USER_CARD_PRACTICE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS user_card_practice (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    card_id BIGINT NOT NULL,
    validated BOOLEAN NOT NULL DEFAULT false,
    validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    direction VARCHAR(20) NOT NULL,
    CONSTRAINT fk_ucp_card FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
);

-- Critical indexes for user_card_practice
-- This table will have high write volume, so indexes are carefully chosen
CREATE INDEX IF NOT EXISTS idx_ucp_user_card ON user_card_practice(user_id, card_id);

-- Composite index for common query: get user's validation history for a card
CREATE INDEX IF NOT EXISTS idx_ucp_user_card_validated ON user_card_practice(user_id, card_id, validated_at DESC);

-- Composite index for querying validations by user and time (for practice history)
CREATE INDEX IF NOT EXISTS idx_ucp_user_validated_at ON user_card_practice(user_id, validated_at DESC);

-- Index for filtering by validation status (for analytics)
CREATE INDEX IF NOT EXISTS idx_ucp_validated ON user_card_practice(validated) WHERE validated = true;

-- Index for card-based queries (e.g., get all validations for a card)
CREATE INDEX IF NOT EXISTS idx_ucp_card_validated_at ON user_card_practice(card_id, validated_at DESC);

-- Partial index for recent validations (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_ucp_recent_validations ON user_card_practice(user_id, card_id, validated_at DESC) 
    WHERE validated_at > NOW() - INTERVAL '30 days';
"#;

/// Full database schema with all tables
pub fn full_schema() -> String {
    format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}",
        TOPICS_SCHEMA,
        DECKS_SCHEMA,
        CARDS_SCHEMA,
        USER_DECK_PRACTICE_SCHEMA,
        USER_CARD_PRACTICE_SCHEMA
    )
}

/// Additional optimization indexes for analytics and reporting
/// These can be added after initial deployment if needed
pub const ANALYTICS_INDEXES: &str = r#"
-- Additional indexes for analytics (add only if needed for reporting queries)

-- For querying user practice statistics
CREATE INDEX IF NOT EXISTS idx_ucp_user_date ON user_card_practice(user_id, DATE(validated_at));

-- For querying deck performance across users
CREATE INDEX IF NOT EXISTS idx_ucp_card_date ON user_card_practice(card_id, DATE(validated_at));
"#;
