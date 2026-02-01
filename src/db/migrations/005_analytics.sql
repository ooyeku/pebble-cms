-- Analytics tables for privacy-preserving traffic tracking

CREATE TABLE IF NOT EXISTS analytics_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    path TEXT NOT NULL,
    referrer_domain TEXT,
    country_code TEXT,
    device_type TEXT CHECK (device_type IN ('desktop', 'mobile', 'tablet')),
    browser_family TEXT,
    session_hash TEXT,
    response_time_ms INTEGER,
    status_code INTEGER,
    content_id INTEGER REFERENCES content(id) ON DELETE SET NULL,
    content_type TEXT
);

CREATE INDEX IF NOT EXISTS idx_events_timestamp ON analytics_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_path ON analytics_events(path);
CREATE INDEX IF NOT EXISTS idx_events_session ON analytics_events(session_hash, timestamp);
CREATE INDEX IF NOT EXISTS idx_events_content ON analytics_events(content_id);

CREATE TABLE IF NOT EXISTS analytics_hourly (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hour TEXT NOT NULL,
    path TEXT NOT NULL,
    content_id INTEGER,
    content_type TEXT,
    pageviews INTEGER DEFAULT 0,
    unique_sessions INTEGER DEFAULT 0,
    avg_response_time_ms INTEGER,
    error_count INTEGER DEFAULT 0,
    UNIQUE(hour, path)
);

CREATE INDEX IF NOT EXISTS idx_hourly_hour ON analytics_hourly(hour);
CREATE INDEX IF NOT EXISTS idx_hourly_content ON analytics_hourly(content_id);

CREATE TABLE IF NOT EXISTS analytics_daily (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,
    total_pageviews INTEGER DEFAULT 0,
    unique_sessions INTEGER DEFAULT 0,
    top_pages TEXT DEFAULT '[]',
    top_posts TEXT DEFAULT '[]',
    referrers TEXT DEFAULT '{}',
    countries TEXT DEFAULT '{}',
    devices TEXT DEFAULT '{}',
    browsers TEXT DEFAULT '{}',
    avg_response_time_ms INTEGER,
    error_rate REAL,
    new_content_views INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_daily_date ON analytics_daily(date);

CREATE TABLE IF NOT EXISTS analytics_content (
    content_id INTEGER PRIMARY KEY REFERENCES content(id) ON DELETE CASCADE,
    total_pageviews INTEGER DEFAULT 0,
    unique_sessions INTEGER DEFAULT 0,
    first_viewed_at TEXT,
    last_viewed_at TEXT,
    avg_time_on_page_seconds INTEGER,
    bounce_rate REAL,
    top_referrers TEXT DEFAULT '[]',
    view_trend TEXT DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS analytics_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
