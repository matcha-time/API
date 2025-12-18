-- Migration: Add language preference fields to users table
-- This allows users to specify their native language and the language they want to learn

ALTER TABLE users
ADD COLUMN native_language CHAR(2),
ADD COLUMN learning_language CHAR(2);

-- Add index for faster filtering by language preferences
CREATE INDEX idx_users_language_preferences ON users (native_language, learning_language) WHERE native_language IS NOT NULL AND learning_language IS NOT NULL;

-- Add comment for documentation
COMMENT ON COLUMN users.native_language IS 'ISO 639-1 language code for user''s native language (e.g., ''en'', ''es'', ''fr'')';
COMMENT ON COLUMN users.learning_language IS 'ISO 639-1 language code for the language user wants to learn (e.g., ''en'', ''es'', ''fr'')';
