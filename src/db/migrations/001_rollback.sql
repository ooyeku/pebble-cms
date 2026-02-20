-- Rollback migration 001: Remove all core tables
-- WARNING: This is DESTRUCTIVE and will delete ALL site data.
-- Requires --force flag to execute.

DROP TRIGGER IF EXISTS update_users_timestamp;
DROP TRIGGER IF EXISTS update_content_timestamp;

DROP INDEX IF EXISTS idx_sessions_expires;
DROP INDEX IF EXISTS idx_sessions_token;
DROP INDEX IF EXISTS idx_content_slug;
DROP INDEX IF EXISTS idx_content_published;
DROP INDEX IF EXISTS idx_content_type;
DROP INDEX IF EXISTS idx_content_status;

DROP TABLE IF EXISTS content_tags;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS media;
DROP TABLE IF EXISTS content;
DROP TABLE IF EXISTS users;
