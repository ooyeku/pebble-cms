-- Content series: ordered groups of posts (e.g., "Building X in Rust, Part 1–5")
CREATE TABLE IF NOT EXISTS content_series (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'published', 'archived')),
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Junction table for series membership with ordering
CREATE TABLE IF NOT EXISTS series_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    series_id INTEGER NOT NULL REFERENCES content_series(id) ON DELETE CASCADE,
    content_id INTEGER NOT NULL REFERENCES content(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    UNIQUE(series_id, content_id)
);

CREATE INDEX IF NOT EXISTS idx_series_slug ON content_series(slug);
CREATE INDEX IF NOT EXISTS idx_series_items_series ON series_items(series_id, position);
CREATE INDEX IF NOT EXISTS idx_series_items_content ON series_items(content_id);
