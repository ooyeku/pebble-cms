-- Rollback migration 005: Remove analytics tables
-- Note: Loses all analytics data

DROP INDEX IF EXISTS idx_daily_date;
DROP INDEX IF EXISTS idx_hourly_content;
DROP INDEX IF EXISTS idx_hourly_hour;
DROP INDEX IF EXISTS idx_events_content;
DROP INDEX IF EXISTS idx_events_session;
DROP INDEX IF EXISTS idx_events_path;
DROP INDEX IF EXISTS idx_events_timestamp;

DROP TABLE IF EXISTS analytics_settings;
DROP TABLE IF EXISTS analytics_content;
DROP TABLE IF EXISTS analytics_daily;
DROP TABLE IF EXISTS analytics_hourly;
DROP TABLE IF EXISTS analytics_events;
