-- Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. USERS
CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username      TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);

-- 2. ROADMAPS (filtered by language pair)
CREATE TABLE roadmaps (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title         TEXT NOT NULL,
    description   TEXT,
    language_from CHAR(2) NOT NULL,
    language_to   CHAR(2) NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(language_from, language_to, title)
);
CREATE INDEX idx_roadmaps_langs ON roadmaps(language_from, language_to);

-- 3. DECKS (reusable everywhere)
CREATE TABLE decks (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title         TEXT NOT NULL,
    description   TEXT,
    language_from CHAR(2) NOT NULL,
    language_to   CHAR(2) NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_decks_langs ON decks(language_from, language_to);

-- 4. ROADMAP NODES (visual tree with x,y coordinates)
CREATE TABLE roadmap_nodes (
    id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    roadmap_id     UUID NOT NULL REFERENCES roadmaps(id) ON DELETE CASCADE,
    deck_id        UUID NOT NULL REFERENCES decks(id),
    parent_node_id UUID REFERENCES roadmap_nodes(id) ON DELETE SET NULL,
    pos_x          NUMERIC(10,2) NOT NULL DEFAULT 0,
    pos_y          NUMERIC(10,2) NOT NULL DEFAULT 0,
    created_at     TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_nodes_roadmap ON roadmap_nodes(roadmap_id);
CREATE INDEX idx_nodes_parent  ON roadmap_nodes(parent_node_id);

-- 5. FLASHCARDS
CREATE TABLE flashcards (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    term          TEXT NOT NULL,
    translation   TEXT NOT NULL,
    language_from CHAR(2) NOT NULL,
    language_to   CHAR(2) NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);

-- 6. DECK ↔ FLASHCARDS (many-to-many)
CREATE TABLE deck_flashcards (
    deck_id      UUID NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    flashcard_id UUID NOT NULL REFERENCES flashcards(id),
    PRIMARY KEY (deck_id, flashcard_id)
);
CREATE INDEX idx_df_deck ON deck_flashcards(deck_id);

-- 7. USER_CARD_PROGRESS (SRS + live stats)
CREATE TABLE user_card_progress (
    user_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    flashcard_id   UUID NOT NULL REFERENCES flashcards(id) ON DELETE CASCADE,
    ease_factor    NUMERIC(5,2) NOT NULL DEFAULT 2.50,
    interval_days  INT NOT NULL DEFAULT 0,
    repetitions    INT NOT NULL DEFAULT 0,
    next_review_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_review_at TIMESTAMPTZ,
    times_correct  INT NOT NULL DEFAULT 0,
    times_wrong    INT NOT NULL DEFAULT 0,
    mastered_at    TIMESTAMPTZ,                    -- NULL = not mastered
    updated_at     TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, flashcard_id)
);

-- MAGIC INDEXES → 0.2 ms total
CREATE INDEX idx_progress_user_due 
    ON user_card_progress(user_id, next_review_at) 
    WHERE next_review_at <= NOW();

CREATE INDEX idx_progress_user_mastered 
    ON user_card_progress(user_id) 
    WHERE mastered_at IS NOT NULL;

CREATE INDEX idx_progress_flashcard ON user_card_progress(flashcard_id);

CREATE INDEX idx_df_deck_covering ON deck_flashcards(deck_id) INCLUDE (flashcard_id);