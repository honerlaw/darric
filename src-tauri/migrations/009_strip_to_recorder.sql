-- Reduce the schema to what a recorder needs: sessions, their recording segments,
-- their transcript lines, and settings. The notes / tasks / tags / chat features
-- were removed from the app, so their tables are dropped rather than left orphaned.
DROP TABLE IF EXISTS session_tags;
DROP TABLE IF EXISTS note_tags;
DROP TABLE IF EXISTS task_tags;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS notes;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS chat_messages;

-- Per-session freeform notes went with the notes feature.
ALTER TABLE sessions DROP COLUMN notes;
