-- Attribute each transcript line to the device that produced it.
--
-- The old `source` column carried CHECK(source IN ('mic','speaker')) and a
-- `speaker_label` guessed by MFCC fingerprinting. SQLite cannot drop or alter a
-- CHECK constraint, so this is the full rebuild: create, copy, drop, rename, and
-- recreate the index. Existing rows are mapped onto synthetic legacy device ids
-- rather than discarded — they are real transcripts, they just predate the
-- device being known.

CREATE TABLE transcript_lines_new (
  id          TEXT PRIMARY KEY,
  session_id  TEXT NOT NULL REFERENCES sessions(id),
  device_id   TEXT NOT NULL,
  device_name TEXT NOT NULL,
  direction   TEXT NOT NULL CHECK(direction IN ('input', 'output')),
  content     TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);

INSERT INTO transcript_lines_new (id, session_id, device_id, device_name, direction, content, recorded_at)
SELECT
  id,
  session_id,
  CASE source WHEN 'speaker' THEN 'legacy-output' ELSE 'legacy-input' END,
  CASE source WHEN 'speaker' THEN 'System audio (pre-upgrade)' ELSE 'Microphone (pre-upgrade)' END,
  CASE source WHEN 'speaker' THEN 'output' ELSE 'input' END,
  content,
  recorded_at
FROM transcript_lines;

DROP TABLE transcript_lines;
ALTER TABLE transcript_lines_new RENAME TO transcript_lines;

CREATE INDEX idx_transcript_session ON transcript_lines(session_id);
CREATE INDEX idx_transcript_device ON transcript_lines(session_id, device_id);
