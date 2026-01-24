# Pebble Analytics

A privacy-preserving, zero-configuration analytics system for Pebble CMS.

## Design Principles

1. **Privacy First** - No personal data collection, no cookies, no fingerprinting
2. **Zero Configuration** - Works automatically out of the box
3. **Self-Contained** - All data stored in the Pebble SQLite database
4. **Modular** - Designed as a standalone library (`pebble-analytics`) that can be extracted
5. **Lightweight** - Minimal performance impact, async batch processing
6. **Useful** - Provides actionable insights without invasive tracking

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Pebble CMS                               │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   pebble-analytics                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │  Collector  │  │  Processor  │  │    Aggregator   │   │  │
│  │  │  (Middleware)│  │  (Async)    │  │    (Scheduled)  │   │  │
│  │  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘   │  │
│  │         │                │                   │            │  │
│  │         ▼                ▼                   ▼            │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │                   Storage Layer                     │  │  │
│  │  │  (SQLite tables, configurable retention)            │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  │         │                                                 │  │
│  │         ▼                                                 │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │                   Query Layer                       │  │  │
│  │  │  (Reports, dashboards, API)                         │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Model

### What We Collect (Privacy-Preserving)

| Data Point | Purpose | Privacy Note |
|------------|---------|--------------|
| Page path | Content performance | No query params with PII |
| Timestamp | Time-based analysis | Rounded to hour for aggregation |
| Referrer domain | Traffic sources | Domain only, no full URL |
| Country | Geographic insights | Derived from IP, IP not stored |
| Device type | Responsive design insights | Mobile/Tablet/Desktop only |
| Browser family | Compatibility planning | Chrome/Firefox/Safari/Other |
| Session hash | Unique visit counting | Non-reversible, daily rotating salt |
| Response time | Performance monitoring | Server-side only |
| Status code | Error tracking | 200, 404, 500, etc. |

### What We Never Collect

- IP addresses
- User agents (raw)
- Cookies or persistent identifiers
- Personal information
- Form data
- Authentication details
- Query parameters (stripped)
- Full referrer URLs

## Database Schema

### Raw Events Table (Short Retention)

```sql
CREATE TABLE analytics_events (
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

CREATE INDEX idx_events_timestamp ON analytics_events(timestamp);
CREATE INDEX idx_events_path ON analytics_events(path);
CREATE INDEX idx_events_session ON analytics_events(session_hash, timestamp);
```

### Hourly Aggregates Table

```sql
CREATE TABLE analytics_hourly (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hour TEXT NOT NULL, -- '2024-01-15T14:00:00Z'
    path TEXT NOT NULL,
    content_id INTEGER,
    content_type TEXT,
    pageviews INTEGER DEFAULT 0,
    unique_sessions INTEGER DEFAULT 0,
    avg_response_time_ms INTEGER,
    error_count INTEGER DEFAULT 0,
    UNIQUE(hour, path)
);

CREATE INDEX idx_hourly_hour ON analytics_hourly(hour);
CREATE INDEX idx_hourly_content ON analytics_hourly(content_id);
```

### Daily Aggregates Table

```sql
CREATE TABLE analytics_daily (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL, -- '2024-01-15'
    
    -- Traffic metrics
    total_pageviews INTEGER DEFAULT 0,
    unique_sessions INTEGER DEFAULT 0,
    
    -- Top content (JSON array)
    top_pages TEXT DEFAULT '[]',
    top_posts TEXT DEFAULT '[]',
    
    -- Traffic sources (JSON object)
    referrers TEXT DEFAULT '{}',
    
    -- Geography (JSON object)
    countries TEXT DEFAULT '{}',
    
    -- Technology (JSON objects)
    devices TEXT DEFAULT '{}',
    browsers TEXT DEFAULT '{}',
    
    -- Performance
    avg_response_time_ms INTEGER,
    error_rate REAL,
    
    -- Content metrics
    new_content_views INTEGER DEFAULT 0, -- Content < 7 days old
    
    UNIQUE(date)
);

CREATE INDEX idx_daily_date ON analytics_daily(date);
```

### Content Performance Table

```sql
CREATE TABLE analytics_content (
    content_id INTEGER PRIMARY KEY REFERENCES content(id) ON DELETE CASCADE,
    total_pageviews INTEGER DEFAULT 0,
    unique_sessions INTEGER DEFAULT 0,
    first_viewed_at TEXT,
    last_viewed_at TEXT,
    avg_time_on_page_seconds INTEGER, -- Estimated from session patterns
    bounce_rate REAL, -- Single-page sessions
    top_referrers TEXT DEFAULT '[]', -- JSON array
    view_trend TEXT DEFAULT '[]' -- Last 30 days daily views
);
```

## Module Structure

```
pebble-analytics/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── collector.rs        # Request data extraction
│   ├── middleware.rs       # Axum/Tower middleware
│   ├── processor.rs        # Async event processing
│   ├── aggregator.rs       # Scheduled aggregation jobs
│   ├── storage.rs          # Database operations
│   ├── query.rs            # Report generation
│   ├── geo.rs              # Country lookup (embedded DB)
│   ├── device.rs           # User-agent parsing
│   ├── privacy.rs          # Hashing, anonymization
│   └── models.rs           # Data structures
└── tests/
```

## Public API

### Initialization

```rust
use pebble_analytics::{Analytics, AnalyticsConfig};

// Zero-config initialization
let analytics = Analytics::new(db.clone());

// Or with custom config
let analytics = Analytics::with_config(db.clone(), AnalyticsConfig {
    enabled: true,
    raw_event_retention_hours: 48,
    session_timeout_minutes: 30,
    excluded_paths: vec!["/admin", "/api", "/static"],
    geo_lookup: true,
    ..Default::default()
});
```

### Middleware Integration

```rust
use pebble_analytics::middleware::AnalyticsLayer;

let app = Router::new()
    .route("/", get(index))
    .layer(AnalyticsLayer::new(analytics.clone()));
```

### Query API

```rust
use pebble_analytics::query::{DateRange, Report};

// Dashboard summary
let summary = analytics.summary(DateRange::Last7Days)?;

// Page performance
let pages = analytics.top_pages(DateRange::Last30Days, 10)?;

// Traffic sources
let referrers = analytics.referrers(DateRange::ThisMonth)?;

// Content-specific stats
let post_stats = analytics.content_stats(content_id)?;

// Real-time (last 30 minutes)
let realtime = analytics.realtime()?;

// Custom query
let report = analytics.query(Report {
    date_range: DateRange::Custom { start, end },
    metrics: vec![Metric::Pageviews, Metric::UniqueSessions],
    dimensions: vec![Dimension::Path, Dimension::Country],
    filters: vec![Filter::ContentType("post")],
    order_by: OrderBy::Pageviews,
    limit: 100,
})?;
```

### Data Structures

```rust
pub struct DashboardSummary {
    pub date_range: DateRange,
    pub total_pageviews: u64,
    pub unique_sessions: u64,
    pub pageviews_change: f64,      // vs previous period
    pub sessions_change: f64,
    pub avg_session_duration: Duration,
    pub bounce_rate: f64,
    pub top_pages: Vec<PageStats>,
    pub top_referrers: Vec<ReferrerStats>,
    pub devices: DeviceBreakdown,
    pub browsers: BrowserBreakdown,
    pub countries: Vec<CountryStats>,
    pub pageviews_over_time: Vec<TimeSeriesPoint>,
}

pub struct PageStats {
    pub path: String,
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub pageviews: u64,
    pub unique_sessions: u64,
    pub avg_time_on_page: Duration,
    pub bounce_rate: f64,
    pub trend: Trend, // Up, Down, Stable
}

pub struct ReferrerStats {
    pub domain: String,
    pub sessions: u64,
    pub percentage: f64,
}

pub struct RealtimeData {
    pub active_sessions: u64,      // Last 5 minutes
    pub pageviews_last_30min: u64,
    pub current_pages: Vec<ActivePage>,
    pub recent_referrers: Vec<String>,
}
```

## Admin Dashboard UI

### Overview Tab

```
┌─────────────────────────────────────────────────────────────────┐
│  Analytics                                    [Last 7 Days ▼]   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
│  │   1,234     │ │    567      │ │   2m 34s    │ │   42%     │ │
│  │  Pageviews  │ │  Sessions   │ │  Avg. Time  │ │  Bounce   │ │
│  │   +12% ↑    │ │   +8% ↑     │ │   -5% ↓     │ │   -3% ↓   │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘ │
│                                                                 │
│  Pageviews Over Time                                            │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │     ╭─╮                                                    │ │
│  │    ╭╯ ╰╮    ╭──╮                          ╭╮              │ │
│  │ ╭──╯   ╰────╯  ╰──────╮    ╭─────────────╯╰──╮           │ │
│  │─╯                      ╰────╯                  ╰───────────│ │
│  │ Mon   Tue   Wed   Thu   Fri   Sat   Sun                   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────┐ ┌─────────────────────────────┐   │
│  │ Top Pages               │ │ Traffic Sources             │   │
│  │─────────────────────────│ │─────────────────────────────│   │
│  │ 1. /posts/ziggurat  324 │ │ Direct           45% ████▌  │   │
│  │ 2. /                 201 │ │ google.com       28% ███    │   │
│  │ 3. /posts/vigil-api 156 │ │ twitter.com      15% ██     │   │
│  │ 4. /pages/about     142 │ │ github.com        8% █      │   │
│  │ 5. /posts           98  │ │ Other             4% ▌      │   │
│  └─────────────────────────┘ └─────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────┐ ┌─────────────────────────────┐   │
│  │ Devices                 │ │ Countries                   │   │
│  │─────────────────────────│ │─────────────────────────────│   │
│  │ Desktop      62% ██████ │ │ 🇺🇸 United States    45%    │   │
│  │ Mobile       31% ███    │ │ 🇬🇧 United Kingdom   12%    │   │
│  │ Tablet        7% █      │ │ 🇩🇪 Germany          8%     │   │
│  └─────────────────────────┘ │ 🇫🇷 France           6%     │   │
│                              │ 🇨🇦 Canada           5%     │   │
│                              └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Content Tab

```
┌─────────────────────────────────────────────────────────────────┐
│  Content Performance                          [Last 30 Days ▼]  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Title              Type   Views  Sessions  Time   Bounce  │  │
│  │─────────────────────────────────────────────────────────  │  │
│  │ Ziggurat Usage     Post   1,234    567    3:24    38% ↓  │  │
│  │ Vigil API Ref      Post     892    412    2:15    45%    │  │
│  │ About              Page     654    423    1:02    62% ↑  │  │
│  │ Pebble Guide       Page     432    234    4:32    28% ↓  │  │
│  │ Getting Started    Post     321    198    2:45    41%    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Content Insights                                               │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 📈 "Ziggurat Usage" is trending (+45% this week)          │  │
│  │ 📉 "About" has high bounce rate - consider adding links   │  │
│  │ ⏱️ "Pebble Guide" has best engagement (4:32 avg time)     │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Realtime Tab

```
┌─────────────────────────────────────────────────────────────────┐
│  Realtime                                        ● Live         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────────────────────────┐   │
│  │       12        │  │ Pageviews (Last 30 minutes)         │   │
│  │                 │  │ ▁▂▃▅▆▇█▆▅▄▃▂▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄▅   │   │
│  │ Active Visitors │  │ 30m ago                         Now │   │
│  └─────────────────┘  └─────────────────────────────────────┘   │
│                                                                 │
│  Active Pages                      Recent Referrers             │
│  ┌────────────────────────────┐   ┌────────────────────────┐   │
│  │ /posts/ziggurat        5   │   │ google.com         3s  │   │
│  │ /                      3   │   │ twitter.com       15s  │   │
│  │ /posts/vigil-api       2   │   │ (direct)          22s  │   │
│  │ /pages/about           2   │   │ github.com        45s  │   │
│  └────────────────────────────┘   └────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Session Tracking (Privacy-Preserving)

### Session Hash Generation

Sessions are tracked using a non-reversible, non-persistent hash:

```rust
fn generate_session_hash(request: &Request) -> String {
    let daily_salt = get_daily_salt(); // Rotates every 24 hours
    let components = format!(
        "{}|{}|{}|{}",
        daily_salt,
        // Anonymized IP: only first two octets for IPv4
        anonymize_ip(request.client_ip()),
        // Simplified user agent (browser family only)
        extract_browser_family(request.user_agent()),
        // Accept-Language header
        request.accept_language().unwrap_or("unknown")
    );
    
    // SHA-256 hash, truncated to 16 chars
    sha256(&components)[..16].to_string()
}
```

This approach:
- Cannot identify individual users
- Cannot track users across days (salt rotates)
- Cannot reverse-engineer original data
- Still allows counting unique sessions within a day

## Geographic Data

### Privacy-Preserving Country Lookup

```rust
fn lookup_country(ip: &str) -> Option<String> {
    // Use embedded MaxMind GeoLite2-Country database
    // Only country-level precision, no city/region
    let country = geo_db.lookup_country(ip)?;
    
    // IP is NEVER stored, only the country code
    Some(country.iso_code.to_string())
}
```

The IP address:
- Is used only for country lookup
- Is never written to disk
- Is never logged
- Is discarded immediately after lookup

## Aggregation Jobs

### Hourly Aggregation (Every Hour)

```rust
async fn aggregate_hourly(analytics: &Analytics) -> Result<()> {
    let hour = current_hour_truncated();
    let previous_hour = hour - Duration::hours(1);
    
    // Aggregate raw events into hourly buckets
    analytics.storage.execute(r#"
        INSERT INTO analytics_hourly (hour, path, content_id, content_type, 
                                       pageviews, unique_sessions, avg_response_time_ms, error_count)
        SELECT 
            ?1 as hour,
            path,
            content_id,
            content_type,
            COUNT(*) as pageviews,
            COUNT(DISTINCT session_hash) as unique_sessions,
            AVG(response_time_ms) as avg_response_time_ms,
            SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as error_count
        FROM analytics_events
        WHERE timestamp >= ?2 AND timestamp < ?1
        GROUP BY path, content_id, content_type
        ON CONFLICT(hour, path) DO UPDATE SET
            pageviews = pageviews + excluded.pageviews,
            unique_sessions = unique_sessions + excluded.unique_sessions
    "#, [hour, previous_hour])?;
    
    // Clean up old raw events (keep 48 hours by default)
    analytics.storage.execute(r#"
        DELETE FROM analytics_events 
        WHERE timestamp < datetime('now', '-48 hours')
    "#)?;
    
    Ok(())
}
```

### Daily Aggregation (Every Day at Midnight)

```rust
async fn aggregate_daily(analytics: &Analytics) -> Result<()> {
    let yesterday = today() - Duration::days(1);
    
    // Compute daily summary from hourly data
    let summary = compute_daily_summary(&analytics.storage, yesterday)?;
    
    // Store aggregated daily stats
    analytics.storage.insert_daily(summary)?;
    
    // Update content performance table
    update_content_stats(&analytics.storage, yesterday)?;
    
    // Clean up old hourly data (keep 90 days)
    analytics.storage.execute(r#"
        DELETE FROM analytics_hourly 
        WHERE hour < datetime('now', '-90 days')
    "#)?;
    
    // Rotate session salt
    rotate_daily_salt()?;
    
    Ok(())
}
```

## Configuration Options

```rust
pub struct AnalyticsConfig {
    /// Enable/disable analytics collection
    pub enabled: bool,
    
    /// Hours to keep raw events before aggregation cleanup
    pub raw_event_retention_hours: u32,
    
    /// Days to keep hourly aggregates
    pub hourly_retention_days: u32,
    
    /// Days to keep daily aggregates (0 = forever)
    pub daily_retention_days: u32,
    
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
    
    /// Paths to exclude from tracking
    pub excluded_paths: Vec<String>,
    
    /// Path prefixes to exclude
    pub excluded_prefixes: Vec<String>,
    
    /// Enable geographic lookup
    pub geo_lookup: bool,
    
    /// Respect Do Not Track header
    pub respect_dnt: bool,
    
    /// Sample rate (1.0 = 100%, 0.1 = 10%)
    pub sample_rate: f64,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            raw_event_retention_hours: 48,
            hourly_retention_days: 90,
            daily_retention_days: 0, // Keep forever
            session_timeout_minutes: 30,
            excluded_paths: vec![
                "/admin".into(),
                "/api".into(),
                "/health".into(),
                "/robots.txt".into(),
                "/favicon.ico".into(),
            ],
            excluded_prefixes: vec![
                "/static".into(),
                "/media".into(),
                "/_".into(),
            ],
            geo_lookup: true,
            respect_dnt: true,
            sample_rate: 1.0,
        }
    }
}
```

## Export Formats

### JSON Export

```rust
let data = analytics.export(DateRange::Last30Days, ExportFormat::Json)?;
```

```json
{
  "exported_at": "2024-01-15T10:30:00Z",
  "date_range": { "start": "2024-01-01", "end": "2024-01-15" },
  "summary": {
    "total_pageviews": 12345,
    "unique_sessions": 5678,
    "avg_session_duration_seconds": 145
  },
  "daily_stats": [...],
  "top_pages": [...],
  "referrers": [...],
  "countries": [...]
}
```

### CSV Export

```rust
let csv = analytics.export(DateRange::Last30Days, ExportFormat::Csv)?;
```

## REST API Endpoints

For headless/API access:

```
GET /api/analytics/summary?range=7d
GET /api/analytics/realtime
GET /api/analytics/pages?range=30d&limit=10
GET /api/analytics/content/:id
GET /api/analytics/referrers?range=30d
GET /api/analytics/export?range=30d&format=json
```

## Performance Considerations

### Async Processing

Events are collected synchronously but processed asynchronously:

```rust
// In middleware - fast, non-blocking
let event = collector.extract(&request, &response);
analytics.queue.send(event); // Async channel, returns immediately

// Background processor
async fn process_events(rx: Receiver<Event>, storage: Storage) {
    let mut batch = Vec::with_capacity(100);
    let mut interval = interval(Duration::from_secs(1));
    
    loop {
        select! {
            Some(event) = rx.recv() => {
                batch.push(event);
                if batch.len() >= 100 {
                    storage.insert_batch(&batch).await;
                    batch.clear();
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    storage.insert_batch(&batch).await;
                    batch.clear();
                }
            }
        }
    }
}
```

### Database Indexes

Critical indexes for query performance:

```sql
-- Event queries
CREATE INDEX idx_events_timestamp ON analytics_events(timestamp);
CREATE INDEX idx_events_path ON analytics_events(path);

-- Hourly rollups
CREATE INDEX idx_hourly_hour ON analytics_hourly(hour);
CREATE INDEX idx_hourly_content ON analytics_hourly(content_id);

-- Daily queries
CREATE INDEX idx_daily_date ON analytics_daily(date);

-- Content performance
CREATE INDEX idx_content_views ON analytics_content(total_pageviews DESC);
```

### Memory Usage

- Event queue: Bounded channel (1000 events max)
- Batch processing: 100 events per write
- GeoIP database: ~5MB embedded
- User-agent parser: Lazy regex compilation

## Migration Path

When upgrading Pebble with analytics:

```sql
-- Migration: Add analytics tables
-- File: migrations/005_analytics.sql

CREATE TABLE IF NOT EXISTS analytics_events (...);
CREATE TABLE IF NOT EXISTS analytics_hourly (...);
CREATE TABLE IF NOT EXISTS analytics_daily (...);
CREATE TABLE IF NOT EXISTS analytics_content (...);

-- Indexes
CREATE INDEX IF NOT EXISTS ...;
```

## Standalone Usage

The analytics module can be extracted and used independently:

```toml
# Cargo.toml
[dependencies]
pebble-analytics = "0.1"
```

```rust
use pebble_analytics::{Analytics, AnalyticsConfig};
use rusqlite::Connection;

// Use with any SQLite database
let conn = Connection::open("my_app.db")?;
let analytics = Analytics::new(conn);
analytics.migrate()?; // Creates tables

// Use with any web framework
// Axum
app.layer(analytics.axum_layer());

// Actix
app.wrap(analytics.actix_middleware());

// Raw tracking
analytics.track(Event {
    path: "/my-page",
    timestamp: Utc::now(),
    ..Default::default()
});
```

## Future Enhancements

1. **Search Analytics** - Track search queries and results
2. **Event Tracking** - Custom events (downloads, outbound links)
3. **Goals/Conversions** - Track completion of defined goals
4. **A/B Testing** - Built-in experiment support
5. **Alerts** - Traffic anomaly detection
6. **Comparison Reports** - Period-over-period analysis
7. **Cohort Analysis** - New vs returning session patterns
8. **Page Flow** - Entry/exit page analysis
