-- Down migration for 0001_init.sql
-- WARNING: This will delete ALL data in the database!

-- Drop helper function
DROP FUNCTION IF EXISTS refresh_deck_progress(UUID, UUID);

-- Drop tables in reverse order (respecting foreign key dependencies)
DROP TABLE IF EXISTS user_activity CASCADE;
DROP TABLE IF EXISTS user_stats CASCADE;
DROP TABLE IF EXISTS user_deck_progress CASCADE;
DROP TABLE IF EXISTS user_card_progress CASCADE;
DROP TABLE IF EXISTS deck_flashcards CASCADE;
DROP TABLE IF EXISTS flashcards CASCADE;
DROP TABLE IF EXISTS roadmap_nodes CASCADE;
DROP TABLE IF EXISTS decks CASCADE;
DROP TABLE IF EXISTS roadmaps CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Drop custom types
DROP TYPE IF EXISTS auth_provider CASCADE;

-- Note: We don't drop the uuid-ossp extension as other databases might use it
-- If you need to drop it: DROP EXTENSION IF EXISTS "uuid-ossp";
