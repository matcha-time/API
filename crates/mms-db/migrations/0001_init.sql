-- Extensions 
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. USERS 
CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username      TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);

-- 2. ROADMAPS (filtered by language pair) 
CREATE TABLE IF NOT EXISTS roadmaps (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title         TEXT NOT NULL,
    description   TEXT,
    language_from CHAR(2) NOT NULL,
    language_to   CHAR(2) NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(language_from, language_to, title)
);
-- Fast lookup: filter roadmaps by language pair
CREATE INDEX IF NOT EXISTS idx_roadmaps_langs ON roadmaps(language_from, language_to);

-- 3. DECKS (reusable everywhere) 
CREATE TABLE IF NOT EXISTS decks (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title         TEXT NOT NULL,
    description   TEXT,
    language_from CHAR(2) NOT NULL,
    language_to   CHAR(2) NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);
-- Fast lookup: filter decks by language pair
CREATE INDEX IF NOT EXISTS idx_decks_langs ON decks(language_from, language_to);

-- 4. ROADMAP NODES (visual tree with x,y coordinates) 
CREATE TABLE IF NOT EXISTS roadmap_nodes (
    id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    roadmap_id     UUID NOT NULL REFERENCES roadmaps(id) ON DELETE CASCADE,
    deck_id        UUID NOT NULL REFERENCES decks(id),
    parent_node_id UUID REFERENCES roadmap_nodes(id) ON DELETE SET NULL,
    pos_x          INT NOT NULL DEFAULT 0,
    pos_y          INT NOT NULL DEFAULT 0,
    created_at     TIMESTAMPTZ DEFAULT NOW()
);
-- Fast lookup: get all nodes for a roadmap
CREATE INDEX IF NOT EXISTS idx_nodes_roadmap ON roadmap_nodes(roadmap_id);

-- 5. FLASHCARDS 
CREATE TABLE IF NOT EXISTS flashcards (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    term          TEXT NOT NULL,
    translation   TEXT NOT NULL,
    language_from CHAR(2) NOT NULL,
    language_to   CHAR(2) NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT unique_flashcard UNIQUE (term, translation, language_from, language_to)
);
-- Fast lookup: find existing flashcard when creating new ones (prevents duplicates)
CREATE INDEX IF NOT EXISTS idx_flashcards_lookup ON flashcards(language_from, language_to, term);

-- 6. DECK ↔ FLASHCARDS (many-to-many) 
CREATE TABLE IF NOT EXISTS deck_flashcards (
    deck_id      UUID NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    flashcard_id UUID NOT NULL REFERENCES flashcards(id),
    PRIMARY KEY (deck_id, flashcard_id)
);
-- Fast lookup: get all flashcards in a deck
CREATE INDEX IF NOT EXISTS idx_df_deck ON deck_flashcards(deck_id) INCLUDE (flashcard_id);

-- 7. USER_CARD_PROGRESS (SRS + live stats) 
CREATE TABLE IF NOT EXISTS user_card_progress (
    user_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    flashcard_id   UUID NOT NULL REFERENCES flashcards(id) ON DELETE CASCADE,
    next_review_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_review_at TIMESTAMPTZ,
    times_correct  INT NOT NULL DEFAULT 0,
    times_wrong    INT NOT NULL DEFAULT 0,
    mastered_at    TIMESTAMPTZ,                    -- NULL = not mastered
    updated_at     TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, flashcard_id)
);

-- Partial index: only mastered cards (for stats queries)
CREATE INDEX IF NOT EXISTS idx_progress_user_mastered
    ON user_card_progress(user_id)
    WHERE mastered_at IS NOT NULL;
-- Covering index: practice session query (~2ms) - filter due cards in WHERE clause of query
CREATE INDEX IF NOT EXISTS idx_practice_session 
    ON user_card_progress(user_id, flashcard_id, next_review_at) 
    INCLUDE (times_correct, times_wrong, last_review_at);

-- 8. USER_DECK_PROGRESS (aggregated stats - updated after each practice session)
CREATE TABLE IF NOT EXISTS user_deck_progress (
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    deck_id           UUID NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    total_cards       INT NOT NULL DEFAULT 0,
    mastered_cards    INT NOT NULL DEFAULT 0,
    cards_due_today   INT NOT NULL DEFAULT 0,
    total_practices   INT NOT NULL DEFAULT 0,
    last_practiced_at TIMESTAMPTZ,
    updated_at        TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, deck_id)
);
-- Fast lookup: load all deck stats for a user (roadmap page)
CREATE INDEX IF NOT EXISTS idx_udp_user ON user_deck_progress(user_id);

-- 9. USER_STATS (streaks and engagement)
CREATE TABLE IF NOT EXISTS user_stats (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    current_streak_days INT NOT NULL DEFAULT 0,
    longest_streak_days INT NOT NULL DEFAULT 0,
    total_reviews       INT NOT NULL DEFAULT 0,
    total_cards_learned INT NOT NULL DEFAULT 0,
    last_review_date    DATE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

-- 10. USER_ACTIVITY (for heatmap)
CREATE TABLE IF NOT EXISTS user_activity (
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_date DATE NOT NULL,
    reviews_count INT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, activity_date)
);
-- Fast lookup: get last 365 days of activity for heatmap (DESC = newest first)
CREATE INDEX IF NOT EXISTS idx_activity_user_date ON user_activity(user_id, activity_date DESC);

-- HELPER FUNCTION: Update deck progress after practice session
CREATE OR REPLACE FUNCTION refresh_deck_progress(p_user_id UUID, p_deck_id UUID)
RETURNS void AS $$
BEGIN
    INSERT INTO user_deck_progress (
        user_id, deck_id, total_cards, mastered_cards, 
        cards_due_today, total_practices, last_practiced_at, updated_at
    )
    SELECT 
        p_user_id,
        p_deck_id,
        COUNT(*) as total_cards,
        COUNT(*) FILTER (WHERE ucp.mastered_at IS NOT NULL) as mastered_cards,
        COUNT(*) FILTER (WHERE ucp.next_review_at <= NOW()) as cards_due_today,
        COALESCE(SUM(ucp.times_correct + ucp.times_wrong), 0) as total_practices,
        MAX(ucp.last_review_at) as last_practiced_at,
        NOW()
    FROM deck_flashcards df
    LEFT JOIN user_card_progress ucp 
        ON df.flashcard_id = ucp.flashcard_id 
        AND ucp.user_id = p_user_id
    WHERE df.deck_id = p_deck_id
    ON CONFLICT (user_id, deck_id) 
    DO UPDATE SET
        total_cards = EXCLUDED.total_cards,
        mastered_cards = EXCLUDED.mastered_cards,
        cards_due_today = EXCLUDED.cards_due_today,
        total_practices = EXCLUDED.total_practices,
        last_practiced_at = EXCLUDED.last_practiced_at,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

