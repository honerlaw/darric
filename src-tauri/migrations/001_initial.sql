CREATE TABLE sessions (
  id          TEXT PRIMARY KEY,
  topic       TEXT,
  started_at  TEXT NOT NULL,
  ended_at    TEXT,
  created_at  TEXT NOT NULL
);

CREATE TABLE transcript_lines (
  id          TEXT PRIMARY KEY,
  session_id  TEXT NOT NULL REFERENCES sessions(id),
  source      TEXT NOT NULL CHECK(source IN ('mic', 'speaker')),
  content     TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX idx_transcript_session ON transcript_lines(session_id);
CREATE INDEX idx_sessions_started ON sessions(started_at DESC);
