CREATE TABLE IF NOT EXISTS recording_segments (
  id          TEXT PRIMARY KEY,
  session_id  TEXT NOT NULL REFERENCES sessions(id),
  started_at  TEXT NOT NULL,
  ended_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_segments_session ON recording_segments(session_id);
