-- Content versioning system
-- Stores full snapshots of content at each save point

CREATE TABLE content_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_id INTEGER NOT NULL REFERENCES content(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    slug TEXT NOT NULL,
    body_markdown TEXT NOT NULL,
    excerpt TEXT,
    featured_image TEXT,
    metadata TEXT DEFAULT '{}',
    tags_json TEXT DEFAULT '[]',
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(content_id, version_number)
);

-- Index for fast version history lookups
CREATE INDEX idx_content_versions_content ON content_versions(content_id, version_number DESC);
