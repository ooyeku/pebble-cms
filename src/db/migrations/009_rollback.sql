-- Rollback migration 009: Remove content series tables
-- Note: Loses all series definitions and membership

DROP INDEX IF EXISTS idx_series_items_content;
DROP INDEX IF EXISTS idx_series_items_series;
DROP INDEX IF EXISTS idx_series_slug;

DROP TABLE IF EXISTS series_items;
DROP TABLE IF EXISTS content_series;
