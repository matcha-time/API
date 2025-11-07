-- =============================================
-- 1. USERS
-- =============================================
CREATE TABLE users (
    id          BIGSERIAL PRIMARY KEY,
    email       TEXT UNIQUE NOT NULL,
    username    TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_users_email ON users(email);

-- =============================================
-- 2. ROADMAPS (global, by language pair)
-- =============================================
CREATE TABLE roadmaps (
    id          BIGSERIAL PRIMARY KEY,
    lang_from   CHAR(2) NOT NULL CHECK (lang_from ~ '^[a-z]{2}$'),  -- e.g.: 'fr'
    lang_to     CHAR(2) NOT NULL CHECK (lang_to ~ '^[a-z]{2}$'),    -- e.g.: 'en'
    title       TEXT NOT NULL,         -- "French → English : A1 to C2"
    description TEXT,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(lang_from, lang_to, title),
    CHECK (lang_from != lang_to)
);
CREATE INDEX idx_roadmaps_langs ON roadmaps(lang_from, lang_to);

-- =============================================
-- 3. DECKS (created by users or admin)
-- =============================================
CREATE TABLE decks (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT REFERENCES users(id) ON DELETE SET NULL, -- NULL = official deck
    lang_from   CHAR(2) NOT NULL,
    lang_to     CHAR(2) NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    is_public   BOOLEAN DEFAULT TRUE,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, title),
    CHECK (lang_from != lang_to)
);
CREATE INDEX idx_decks_langs ON decks(lang_from, lang_to);
CREATE INDEX idx_decks_user ON decks(user_id);

-- =============================================
-- 4. ROADMAP NODES (tree)
-- =============================================
CREATE TABLE roadmap_nodes (
    id                BIGSERIAL PRIMARY KEY,
    roadmap_id        BIGINT NOT NULL REFERENCES roadmaps(id) ON DELETE CASCADE,
    deck_id           BIGINT NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    parent_id         BIGINT REFERENCES roadmap_nodes(id) ON DELETE SET NULL,
    position_x        INT NOT NULL DEFAULT 0,
    position_y        INT NOT NULL DEFAULT 0,
    unlock_threshold  NUMERIC(5,2) DEFAULT 80.00,   -- % mastery required
    created_at        TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(roadmap_id, deck_id)
);
CREATE INDEX idx_nodes_roadmap ON roadmap_nodes(roadmap_id);
CREATE INDEX idx_nodes_parent ON roadmap_nodes(parent_id);
CREATE INDEX idx_nodes_deck ON roadmap_nodes(deck_id);

-- =============================================
-- 5. CARDS
-- =============================================
CREATE TABLE cards (
    id          BIGSERIAL PRIMARY KEY,
    deck_id     BIGINT NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    front       TEXT NOT NULL,   -- source word
    back        TEXT NOT NULL,   -- translation
    example     TEXT,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_cards_deck ON cards(deck_id);

-- =============================================
-- 6. CARD_SRS (SRS state per user)
-- =============================================
CREATE TABLE card_srs (
    card_id       BIGINT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    interval      INT DEFAULT 0,
    ease_factor   NUMERIC(4,2) DEFAULT 2.50,
    reps          INT DEFAULT 0,
    lapses        INT DEFAULT 0,
    due           TIMESTAMPTZ DEFAULT NOW(),
    last_review   TIMESTAMPTZ,
    mastered      BOOLEAN DEFAULT FALSE,
    correct       INT DEFAULT 0,
    incorrect     INT DEFAULT 0,
    streak        INT DEFAULT 0,
    total_time_ms INT DEFAULT 0,
    PRIMARY KEY (card_id, user_id)
);
CREATE INDEX idx_srs_user_due ON card_srs(user_id, due) WHERE mastered = FALSE;
CREATE INDEX idx_srs_user ON card_srs(user_id);

-- =============================================
-- 7. REVIEWS (history, partitioned by month)
-- =============================================
CREATE TABLE reviews (
    id           BIGSERIAL,
    card_id      BIGINT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating       SMALLINT NOT NULL CHECK (rating IN (1,2,3,4)), -- 1=again 4=easy
    time_ms      INT NOT NULL,
    reviewed_at  TIMESTAMPTZ DEFAULT NOW()
) PARTITION BY RANGE (reviewed_at);

-- Example partitions (create via monthly script)
CREATE TABLE reviews_y2025m11 PARTITION OF reviews FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
CREATE TABLE reviews_y2025m12 PARTITION OF reviews FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');
-- Add indexes on partitions if needed, or globally:
CREATE INDEX idx_reviews_user ON reviews(user_id, reviewed_at DESC);
CREATE INDEX idx_reviews_card ON reviews(card_id);

-- =============================================
-- 8. USER_DECK_PROGRESS (new: progress per deck for fast stats)
-- =============================================
CREATE TABLE user_deck_progress (
    user_id          BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    deck_id          BIGINT NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    progress_percent NUMERIC(5,2) DEFAULT 0.00,  -- % of cards mastered
    total_cards      INT DEFAULT 0,
    mastered_cards   INT DEFAULT 0,
    last_updated     TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, deck_id)
);
CREATE INDEX idx_udp_user ON user_deck_progress(user_id);
CREATE INDEX idx_udp_deck ON user_deck_progress(deck_id);

-- Trigger to update user_deck_progress on changes to card_srs.mastered
CREATE OR REPLACE FUNCTION update_deck_progress() RETURNS TRIGGER AS $$
BEGIN
    WITH stats AS (
        SELECT COUNT(*) AS total,
               SUM(CASE WHEN mastered THEN 1 ELSE 0 END) AS mastered
        FROM card_srs
        WHERE user_id = NEW.user_id AND card_id IN (SELECT id FROM cards WHERE deck_id = (SELECT deck_id FROM cards WHERE id = NEW.card_id))
    )
    UPDATE user_deck_progress
    SET progress_percent = (stats.mastered::NUMERIC / stats.total) * 100,
        total_cards = stats.total,
        mastered_cards = stats.mastered,
        last_updated = NOW()
    WHERE user_id = NEW.user_id AND deck_id = (SELECT deck_id FROM cards WHERE id = NEW.card_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trig_card_srs_update
AFTER INSERT OR UPDATE OF mastered ON card_srs
FOR EACH ROW EXECUTE FUNCTION update_deck_progress();

-- =============================================
-- 9. USER_ROADMAP_PROGRESS (user progress on roadmap)
-- =============================================
CREATE TABLE user_roadmap_progress (
    user_id          BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    roadmap_id       BIGINT NOT NULL REFERENCES roadmaps(id) ON DELETE CASCADE,
    current_node_id  BIGINT REFERENCES roadmap_nodes(id),
    unlocked_nodes   JSONB DEFAULT '[]'::jsonb,   -- array of unlocked IDs
    global_progress  NUMERIC(5,2) DEFAULT 0.00,
    started_at       TIMESTAMPTZ DEFAULT NOW(),
    completed_at     TIMESTAMPTZ,
    PRIMARY KEY (user_id, roadmap_id)
);
CREATE INDEX idx_urp_roadmap ON user_roadmap_progress(roadmap_id);

-- Trigger to update global_progress on changes to deck progress (weighted sum, e.g.: average of decks)
CREATE OR REPLACE FUNCTION update_roadmap_progress() RETURNS TRIGGER AS $$
BEGIN
    WITH avg_prog AS (
        SELECT AVG(progress_percent) AS avg
        FROM user_deck_progress udp
        JOIN roadmap_nodes rn ON rn.deck_id = udp.deck_id
        WHERE udp.user_id = NEW.user_id AND rn.roadmap_id = (SELECT roadmap_id FROM roadmap_nodes WHERE deck_id = NEW.deck_id)
    )
    UPDATE user_roadmap_progress
    SET global_progress = avg_prog.avg,
        completed_at = CASE WHEN avg_prog.avg = 100 THEN NOW() ELSE completed_at END
    WHERE user_id = NEW.user_id AND roadmap_id = (SELECT roadmap_id FROM roadmap_nodes WHERE deck_id = NEW.deck_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trig_deck_progress_update
AFTER UPDATE ON user_deck_progress
FOR EACH ROW EXECUTE FUNCTION update_roadmap_progress();

-- =============================================
-- 10. MATERIALIZED VIEW : complete roadmap + user status (BLAZING FAST)
-- =============================================
CREATE MATERIALIZED VIEW user_roadmap_full AS
SELECT
    urp.user_id,
    r.id               AS roadmap_id,
    r.title            AS roadmap_title,
    r.lang_from,
    r.lang_to,
    n.id               AS node_id,
    n.deck_id,
    d.title            AS deck_title,
    n.parent_id,
    n.position_x,
    n.position_y,
    n.unlock_threshold,
    COALESCE(p.progress_percent, 0) AS progress_percent,
    (COALESCE(p.progress_percent, 0) >= n.unlock_threshold) OR
    (n.id = ANY(urp.unlocked_nodes::bigint[])) AS is_unlocked,
    urp.current_node_id,
    urp.global_progress
FROM roadmaps r
LEFT JOIN roadmap_nodes n ON n.roadmap_id = r.id
LEFT JOIN decks d ON d.id = n.deck_id
LEFT JOIN user_roadmap_progress urp ON urp.roadmap_id = r.id
LEFT JOIN user_deck_progress p ON p.deck_id = n.deck_id AND p.user_id = urp.user_id
LEFT JOIN users u ON u.id = urp.user_id;  -- Optimized: no full CROSS JOIN, join on urp to limit

CREATE UNIQUE INDEX idx_user_roadmap_full_unique ON user_roadmap_full(user_id, roadmap_id, node_id);
-- Refresh: REFRESH MATERIALIZED VIEW CONCURRENTLY user_roadmap_full; (via cron every 1-2 min)

-- =============================================
-- 11. ULTRA-FAST FRONTEND QUERIES
-- =============================================

-- Complete roadmap for a user + unlock status
SELECT * FROM user_roadmap_full
WHERE user_id = $1 AND roadmap_id = $2
ORDER BY position_y, position_x;

-- Cards due today in a deck (for fast sessions)
SELECT c.*, cs.*, d.title AS deck_title
FROM card_srs cs
JOIN cards c ON c.id = cs.card_id
JOIN decks d ON d.id = c.deck_id
WHERE cs.user_id = $1
  AND c.deck_id = $2
  AND cs.due <= NOW()
  AND cs.mastered = FALSE
ORDER BY cs.due
LIMIT 50;

-- Stats of a deck for a user
SELECT * FROM user_deck_progress
WHERE user_id = $1 AND deck_id = $2;