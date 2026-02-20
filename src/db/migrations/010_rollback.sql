-- Rollback migration 010: Remove API tokens and webhooks tables
-- Note: Loses all API tokens and webhook configurations

DROP TABLE IF EXISTS webhook_deliveries;
DROP TABLE IF EXISTS webhooks;
DROP TABLE IF EXISTS api_tokens;
