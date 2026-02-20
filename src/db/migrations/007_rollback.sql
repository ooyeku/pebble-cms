-- Rollback migration 007: Remove audit logs table
-- Note: Loses all audit history

DROP INDEX IF EXISTS idx_audit_filter;
DROP INDEX IF EXISTS idx_audit_status;
DROP INDEX IF EXISTS idx_audit_entity;
DROP INDEX IF EXISTS idx_audit_category;
DROP INDEX IF EXISTS idx_audit_action;
DROP INDEX IF EXISTS idx_audit_user;
DROP INDEX IF EXISTS idx_audit_timestamp;

DROP TABLE IF EXISTS audit_logs;
