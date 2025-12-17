-- Cleanup script to remove ALL data (including old Japanese data)
-- Run this to delete all data from flashcards, decks, roadmaps, and roadmap_nodes

BEGIN;

-- Delete ALL roadmap nodes
DELETE FROM roadmap_nodes;

-- Delete ALL roadmaps
DELETE FROM roadmaps;

-- Delete ALL deck_flashcards associations
DELETE FROM deck_flashcards;

-- Delete ALL decks
DELETE FROM decks;

-- Delete ALL flashcards
DELETE FROM flashcards;

COMMIT;

-- Verify cleanup
SELECT 'Flashcards remaining:' AS info, COUNT(*) AS count FROM flashcards
UNION ALL
SELECT 'Decks remaining:', COUNT(*) FROM decks
UNION ALL
SELECT 'Roadmaps remaining:', COUNT(*) FROM roadmaps
UNION ALL
SELECT 'Roadmap nodes remaining:', COUNT(*) FROM roadmap_nodes;
