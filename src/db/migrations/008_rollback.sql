-- Rollback migration 008: Remove preview tokens table

DROP INDEX IF EXISTS idx_preview_tokens_expires;
DROP INDEX IF EXISTS idx_preview_tokens_token;
DROP TABLE IF EXISTS preview_tokens;
