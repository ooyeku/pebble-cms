CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    username TEXT,
    user_role TEXT,
    action TEXT NOT NULL,
    category TEXT NOT NULL,
    entity_type TEXT,
    entity_id INTEGER,
    entity_title TEXT,
    ip_address TEXT,
    user_agent TEXT,
    status TEXT NOT NULL DEFAULT 'success',
    error_message TEXT,
    changes TEXT,
    metadata TEXT DEFAULT '{}'
);

CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_user ON audit_logs(user_id, timestamp DESC);
CREATE INDEX idx_audit_action ON audit_logs(action, timestamp DESC);
CREATE INDEX idx_audit_category ON audit_logs(category, timestamp DESC);
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_status ON audit_logs(status) WHERE status = 'failure';
CREATE INDEX idx_audit_filter ON audit_logs(category, action, timestamp DESC);
