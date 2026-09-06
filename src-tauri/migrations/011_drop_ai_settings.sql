-- The AI chat feature was removed in 009 but its settings rows were not, so an
-- API key entered before the removal has been sitting in `settings` with
-- nothing reading it. A secret at rest for no reason: drop every key the
-- feature owned.
DELETE FROM settings WHERE key LIKE 'ai.%';
