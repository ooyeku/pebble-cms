-- Add scheduled_at column and update status constraint
-- SQLite doesn't support ALTER CONSTRAINT, so we recreate the table

CREATE TABLE content_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'post' CHECK (content_type IN ('post', 'page', 'snippet')),
    body_markdown TEXT NOT NULL DEFAULT '',
    body_html TEXT NOT NULL DEFAULT '',
    excerpt TEXT,
    featured_image TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'scheduled', 'published', 'archived')),
    scheduled_at TEXT,
    published_at TEXT,
    author_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    metadata TEXT DEFAULT '{}',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO content_new SELECT id, slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, NULL, published_at, author_id, metadata, created_at, updated_at FROM content;

DROP TABLE content;
ALTER TABLE content_new RENAME TO content;

CREATE INDEX idx_content_status ON content(status);
CREATE INDEX idx_content_type ON content(content_type);
CREATE INDEX idx_content_published ON content(published_at DESC);
CREATE INDEX idx_content_slug ON content(slug);
CREATE INDEX idx_content_scheduled ON content(scheduled_at) WHERE scheduled_at IS NOT NULL AND status = 'scheduled';

CREATE TRIGGER update_content_timestamp
AFTER UPDATE ON content
BEGIN
    UPDATE content SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
