-- Rollback migration 002: Remove FTS virtual table and triggers
-- Safe: FTS index can be rebuilt with `pebble rerender`

DROP TRIGGER IF EXISTS content_fts_delete;
DROP TRIGGER IF EXISTS content_fts_update;
DROP TRIGGER IF EXISTS content_fts_insert;
DROP TABLE IF EXISTS content_fts;
