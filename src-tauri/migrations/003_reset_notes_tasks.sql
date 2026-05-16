-- Drop and recreate notes/tasks so the schema is guaranteed correct
-- regardless of what a previous app version created.
DROP TABLE IF EXISTS notes;
CREATE TABLE notes (
  id         TEXT PRIMARY KEY,
  title      TEXT NOT NULL DEFAULT '',
  body       TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_notes_updated ON notes(updated_at DESC);

DROP TABLE IF EXISTS tasks;
CREATE TABLE tasks (
  id         TEXT PRIMARY KEY,
  title      TEXT NOT NULL,
  col        TEXT NOT NULL CHECK(col IN ('todo', 'doing', 'done')) DEFAULT 'todo',
  position   INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_tasks_col_pos ON tasks(col, position);
