-- Rollback migration 006: Remove content versions table
-- Note: Loses all version history

DROP INDEX IF EXISTS idx_content_versions_content;
DROP TABLE IF EXISTS content_versions;
