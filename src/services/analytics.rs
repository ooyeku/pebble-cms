use crate::Database;
use anyhow::Result;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

/// Analytics configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
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
                "/health".into(),
                "/robots.txt".into(),
                "/favicon.ico".into(),
            ],
            excluded_prefixes: vec![
                "/admin".into(),
                "/api".into(),
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

impl AnalyticsConfig {
    /// Check if a path should be tracked
    pub fn should_track(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check exact path matches
        if self.excluded_paths.iter().any(|p| p == path) {
            return false;
        }

        // Check prefix matches
        if self
            .excluded_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            return false;
        }

        // Check sample rate
        if self.sample_rate < 1.0 {
            use rand::Rng;
            if rand::thread_rng().gen::<f64>() > self.sample_rate {
                return false;
            }
        }

        true
    }

    /// Check if DNT should be respected
    pub fn should_respect_dnt(&self, dnt_header: Option<&str>) -> bool {
        if !self.respect_dnt {
            return false;
        }
        matches!(dnt_header, Some("1"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub path: String,
    pub referrer_domain: Option<String>,
    pub country_code: Option<String>,
    pub device_type: DeviceType,
    pub browser_family: String,
    pub session_hash: String,
    pub response_time_ms: Option<i64>,
    pub status_code: u16,
    pub content_id: Option<i64>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    #[default]
    Desktop,
    Mobile,
    Tablet,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Desktop => write!(f, "desktop"),
            DeviceType::Mobile => write!(f, "mobile"),
            DeviceType::Tablet => write!(f, "tablet"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub total_pageviews: i64,
    pub unique_sessions: i64,
    pub pageviews_change: f64,
    pub sessions_change: f64,
    pub avg_response_time_ms: i64,
    pub error_rate: f64,
    pub top_pages: Vec<PageStats>,
    pub top_referrers: Vec<ReferrerStats>,
    pub devices: HashMap<String, i64>,
    pub browsers: HashMap<String, i64>,
    pub total_devices: i64,
    pub total_browsers: i64,
    pub countries: Vec<CountryStats>,
    pub pageviews_over_time: Vec<TimeSeriesPoint>,
    pub pageviews_max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStats {
    pub path: String,
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub pageviews: i64,
    pub unique_sessions: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferrerStats {
    pub domain: String,
    pub sessions: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountryStats {
    pub code: String,
    pub name: String,
    pub sessions: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeSeriesPoint {
    pub timestamp: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RealtimeStats {
    pub active_sessions: i64,
    pub pageviews_30min: i64,
    pub current_pages: Vec<ActivePage>,
    pub recent_referrers: Vec<RecentReferrer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivePage {
    pub path: String,
    pub visitors: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentReferrer {
    pub domain: String,
    pub seconds_ago: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentStats {
    pub content_id: i64,
    pub total_pageviews: i64,
    pub unique_sessions: i64,
    pub first_viewed_at: Option<String>,
    pub last_viewed_at: Option<String>,
    pub view_trend: Vec<i64>,
    pub top_referrers: Vec<ReferrerStats>,
    pub bounce_rate: f64,
}

/// Content performance data for dashboard
#[derive(Debug, Clone, Serialize)]
pub struct ContentPerformance {
    pub content_id: i64,
    pub title: String,
    pub content_type: String,
    pub slug: String,
    pub pageviews: i64,
    pub unique_sessions: i64,
    pub avg_time_seconds: i64,
    pub bounce_rate: f64,
    pub trend: String, // "up", "down", "stable"
    pub trend_percent: f64,
}

/// Daily aggregation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub total_pageviews: i64,
    pub unique_sessions: i64,
    pub top_pages: Vec<PageStats>,
    pub top_posts: Vec<PageStats>,
    pub referrers: HashMap<String, i64>,
    pub countries: HashMap<String, i64>,
    pub devices: HashMap<String, i64>,
    pub browsers: HashMap<String, i64>,
    pub avg_response_time_ms: i64,
    pub error_rate: f64,
    pub new_content_views: i64,
}

/// Export format options
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Exported analytics data
#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsExport {
    pub exported_at: String,
    pub date_range: DateRange,
    pub summary: ExportSummary,
    pub daily_stats: Vec<DailyStats>,
    pub top_pages: Vec<PageStats>,
    pub referrers: Vec<ReferrerStats>,
    pub countries: Vec<CountryStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportSummary {
    pub total_pageviews: i64,
    pub unique_sessions: i64,
    pub avg_response_time_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

pub struct Analytics {
    db: Database,
    config: AnalyticsConfig,
}

impl Analytics {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            config: AnalyticsConfig::default(),
        }
    }

    pub fn with_config(db: Database, config: AnalyticsConfig) -> Self {
        Self { db, config }
    }

    pub fn config(&self) -> &AnalyticsConfig {
        &self.config
    }

    /// Check if tracking should be performed for this request
    pub fn should_track(&self, path: &str, dnt_header: Option<&str>) -> bool {
        if self.config.should_respect_dnt(dnt_header) {
            return false;
        }
        self.config.should_track(path)
    }

    pub fn record_event(&self, event: &AnalyticsEvent) -> Result<()> {
        let conn = self.db.get()?;
        conn.execute(
            r#"
            INSERT INTO analytics_events
                (path, referrer_domain, country_code, device_type, browser_family,
                 session_hash, response_time_ms, status_code, content_id, content_type)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            rusqlite::params![
                event.path,
                event.referrer_domain,
                event.country_code,
                event.device_type.to_string(),
                event.browser_family,
                event.session_hash,
                event.response_time_ms,
                event.status_code as i64,
                event.content_id,
                event.content_type,
            ],
        )?;

        // Update content analytics if we have a content_id
        if let Some(content_id) = event.content_id {
            self.update_content_analytics(content_id, &event.session_hash, event.referrer_domain.as_deref())?;
        }

        Ok(())
    }

    /// Update the analytics_content table for a specific content item
    fn update_content_analytics(&self, content_id: i64, session_hash: &str, referrer: Option<&str>) -> Result<()> {
        let conn = self.db.get()?;
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Check if this is a new session for this content
        let is_new_session: bool = conn
            .query_row(
                r#"
                SELECT COUNT(*) = 0
                FROM analytics_events
                WHERE content_id = ?1 AND session_hash = ?2
                  AND timestamp >= datetime('now', '-30 minutes')
                  AND id != (SELECT MAX(id) FROM analytics_events WHERE content_id = ?1 AND session_hash = ?2)
                "#,
                rusqlite::params![content_id, session_hash],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(true);

        // Upsert into analytics_content
        conn.execute(
            r#"
            INSERT INTO analytics_content (content_id, total_pageviews, unique_sessions, first_viewed_at, last_viewed_at)
            VALUES (?1, 1, ?2, ?3, ?3)
            ON CONFLICT(content_id) DO UPDATE SET
                total_pageviews = total_pageviews + 1,
                unique_sessions = unique_sessions + ?2,
                last_viewed_at = ?3
            "#,
            rusqlite::params![content_id, if is_new_session { 1 } else { 0 }, &now],
        )?;

        // Update top referrers for this content if we have a referrer
        if let Some(domain) = referrer {
            let existing_referrers: String = conn
                .query_row(
                    "SELECT top_referrers FROM analytics_content WHERE content_id = ?1",
                    [content_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| "[]".to_string());

            let mut referrers: Vec<(String, i64)> = serde_json::from_str(&existing_referrers).unwrap_or_default();

            // Find and update or add the referrer
            if let Some(entry) = referrers.iter_mut().find(|(d, _)| d == domain) {
                entry.1 += 1;
            } else {
                referrers.push((domain.to_string(), 1));
            }

            // Sort by count and keep top 10
            referrers.sort_by(|a, b| b.1.cmp(&a.1));
            referrers.truncate(10);

            let referrers_json = serde_json::to_string(&referrers)?;
            conn.execute(
                "UPDATE analytics_content SET top_referrers = ?1 WHERE content_id = ?2",
                rusqlite::params![referrers_json, content_id],
            )?;
        }

        Ok(())
    }

    pub fn get_summary(&self, days: i64) -> Result<DashboardSummary> {
        let conn = self.db.get()?;

        let now = chrono::Utc::now();
        let cutoff = (now - chrono::TimeDelta::days(days))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let prev_cutoff = (now - chrono::TimeDelta::days(days * 2))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        tracing::debug!("Analytics query: days={}, cutoff={}", days, cutoff);

        // Query both analytics_events (recent) and analytics_hourly (aggregated) tables
        let (events_pageviews, events_sessions, events_avg_response, events_errors): (
            i64,
            i64,
            Option<f64>,
            i64,
        ) = conn
            .query_row(
                r#"
                SELECT
                    COUNT(*) as pageviews,
                    COUNT(DISTINCT session_hash) as sessions,
                    AVG(response_time_ms) as avg_response,
                    SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as errors
                FROM analytics_events
                WHERE timestamp >= ?1
                "#,
                [&cutoff],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or((0, 0, None, 0));

        let (hourly_pageviews, hourly_sessions, hourly_avg_response, hourly_errors): (
            i64,
            i64,
            Option<f64>,
            i64,
        ) = conn
            .query_row(
                r#"
                SELECT
                    COALESCE(SUM(pageviews), 0) as pageviews,
                    COALESCE(SUM(unique_sessions), 0) as sessions,
                    AVG(avg_response_time_ms) as avg_response,
                    COALESCE(SUM(error_count), 0) as errors
                FROM analytics_hourly
                WHERE hour >= ?1
                "#,
                [&cutoff],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or((0, 0, None, 0));

        let total_pageviews = events_pageviews + hourly_pageviews;
        let unique_sessions = events_sessions + hourly_sessions;
        let avg_response_time = events_avg_response
            .or(hourly_avg_response)
            .map(|v| v as i64);
        let error_count = events_errors + hourly_errors;

        let (prev_events_pv, prev_events_sess): (i64, i64) = conn
            .query_row(
                r#"
                SELECT
                    COUNT(*) as pageviews,
                    COUNT(DISTINCT session_hash) as sessions
                FROM analytics_events
                WHERE timestamp >= ?1 AND timestamp < ?2
                "#,
                [&prev_cutoff, &cutoff],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0, 0));

        let (prev_hourly_pv, prev_hourly_sess): (i64, i64) = conn
            .query_row(
                r#"
                SELECT
                    COALESCE(SUM(pageviews), 0) as pageviews,
                    COALESCE(SUM(unique_sessions), 0) as sessions
                FROM analytics_hourly
                WHERE hour >= ?1 AND hour < ?2
                "#,
                [&prev_cutoff, &cutoff],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0, 0));

        let prev_pageviews = prev_events_pv + prev_hourly_pv;
        let prev_sessions = prev_events_sess + prev_hourly_sess;

        let pageviews_change = if prev_pageviews > 0 {
            ((total_pageviews - prev_pageviews) as f64 / prev_pageviews as f64) * 100.0
        } else {
            0.0
        };

        let sessions_change = if prev_sessions > 0 {
            ((unique_sessions - prev_sessions) as f64 / prev_sessions as f64) * 100.0
        } else {
            0.0
        };

        let error_rate = if total_pageviews > 0 {
            (error_count as f64 / total_pageviews as f64) * 100.0
        } else {
            0.0
        };

        // Top pages from both tables combined
        let mut stmt = conn.prepare(
            r#"
            SELECT path, SUM(views) as total_views, SUM(sessions) as total_sessions
            FROM (
                SELECT path, COUNT(*) as views, COUNT(DISTINCT session_hash) as sessions
                FROM analytics_events
                WHERE timestamp >= ?1
                GROUP BY path
                UNION ALL
                SELECT path, pageviews as views, unique_sessions as sessions
                FROM analytics_hourly
                WHERE hour >= ?1
            )
            GROUP BY path
            ORDER BY total_views DESC
            LIMIT 10
            "#,
        )?;
        let top_pages: Vec<PageStats> = stmt
            .query_map([&cutoff], |row| {
                Ok(PageStats {
                    path: row.get(0)?,
                    title: None,
                    content_type: None,
                    pageviews: row.get(1)?,
                    unique_sessions: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut stmt = conn.prepare(
            r#"
            SELECT referrer_domain, COUNT(DISTINCT session_hash) as sessions
            FROM analytics_events
            WHERE timestamp >= ?1 AND referrer_domain IS NOT NULL
            GROUP BY referrer_domain
            ORDER BY sessions DESC
            LIMIT 10
            "#,
        )?;
        let referrer_rows: Vec<(String, i64)> = stmt
            .query_map([&cutoff], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let referrer_total: i64 = referrer_rows.iter().map(|(_, s)| s).sum();
        let top_referrers: Vec<ReferrerStats> = referrer_rows
            .into_iter()
            .map(|(domain, sessions)| ReferrerStats {
                domain,
                sessions,
                percentage: if referrer_total > 0 {
                    (sessions as f64 / referrer_total as f64) * 100.0
                } else {
                    0.0
                },
            })
            .collect();

        let mut devices: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT device_type, COUNT(*) as count
            FROM analytics_events
            WHERE timestamp >= ?1
            GROUP BY device_type
            ORDER BY count DESC
            LIMIT 20
            "#,
        )?;
        for row in stmt.query_map([&cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            if let Ok((device, count)) = row {
                devices.insert(device, count);
            }
        }

        let mut browsers: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT browser_family, COUNT(*) as count
            FROM analytics_events
            WHERE timestamp >= ?1
            GROUP BY browser_family
            ORDER BY count DESC
            LIMIT 50
            "#,
        )?;
        for row in stmt.query_map([&cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            if let Ok((browser, count)) = row {
                browsers.insert(browser, count);
            }
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT country_code, COUNT(DISTINCT session_hash) as sessions
            FROM analytics_events
            WHERE timestamp >= ?1 AND country_code IS NOT NULL
            GROUP BY country_code
            ORDER BY sessions DESC
            LIMIT 10
            "#,
        )?;
        let country_rows: Vec<(String, i64)> = stmt
            .query_map([&cutoff], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let country_total: i64 = country_rows.iter().map(|(_, s)| s).sum();
        let countries: Vec<CountryStats> = country_rows
            .into_iter()
            .map(|(code, sessions)| CountryStats {
                name: country_name(&code),
                code,
                sessions,
                percentage: if country_total > 0 {
                    (sessions as f64 / country_total as f64) * 100.0
                } else {
                    0.0
                },
            })
            .collect();

        // Pageviews over time from both tables
        let mut stmt = conn.prepare(
            r#"
            SELECT day, SUM(views) as total_views
            FROM (
                SELECT date(timestamp) as day, COUNT(*) as views
                FROM analytics_events
                WHERE timestamp >= ?1
                GROUP BY day
                UNION ALL
                SELECT date(hour) as day, pageviews as views
                FROM analytics_hourly
                WHERE hour >= ?1
            )
            GROUP BY day
            ORDER BY day ASC
            "#,
        )?;
        let pageviews_over_time: Vec<TimeSeriesPoint> = stmt
            .query_map([&cutoff], |row| {
                Ok(TimeSeriesPoint {
                    timestamp: row.get(0)?,
                    value: row.get::<_, i64>(1)?,
                })
            })?
            .filter_map(|r| {
                r.map_err(|e| tracing::error!("Pageviews over time query error: {}", e))
                    .ok()
            })
            .collect();

        let pageviews_max = pageviews_over_time
            .iter()
            .map(|p| p.value)
            .max()
            .unwrap_or(1)
            .max(1);

        let total_devices: i64 = devices.values().sum();
        let total_browsers: i64 = browsers.values().sum();

        Ok(DashboardSummary {
            total_pageviews,
            unique_sessions,
            pageviews_change,
            sessions_change,
            avg_response_time_ms: avg_response_time.unwrap_or(0),
            error_rate,
            top_pages,
            top_referrers,
            devices,
            browsers,
            total_devices,
            total_browsers,
            countries,
            pageviews_over_time,
            pageviews_max,
        })
    }

    pub fn get_realtime(&self) -> Result<RealtimeStats> {
        let conn = self.db.get()?;

        let now = chrono::Utc::now();
        let five_min_ago = (now - chrono::TimeDelta::minutes(5))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let thirty_min_ago = (now - chrono::TimeDelta::minutes(30))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let active_sessions: i64 = conn
            .query_row(
                r#"
                SELECT COUNT(DISTINCT session_hash)
                FROM analytics_events
                WHERE timestamp >= ?1
                "#,
                [&five_min_ago],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let pageviews_30min: i64 = conn
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM analytics_events
                WHERE timestamp >= ?1
                "#,
                [&thirty_min_ago],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let mut stmt = conn.prepare(
            r#"
            SELECT path, COUNT(DISTINCT session_hash) as visitors
            FROM analytics_events
            WHERE timestamp >= ?1
            GROUP BY path
            ORDER BY visitors DESC
            LIMIT 10
            "#,
        )?;
        let current_pages: Vec<ActivePage> = stmt
            .query_map([&five_min_ago], |row| {
                Ok(ActivePage {
                    path: row.get(0)?,
                    visitors: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Recent referrers with time ago
        let mut stmt = conn.prepare(
            r#"
            SELECT referrer_domain,
                   CAST((julianday('now') - julianday(timestamp)) * 86400 AS INTEGER) as seconds_ago
            FROM analytics_events
            WHERE timestamp >= ?1 AND referrer_domain IS NOT NULL
            ORDER BY timestamp DESC
            LIMIT 5
            "#,
        )?;
        let recent_referrers: Vec<RecentReferrer> = stmt
            .query_map([&five_min_ago], |row| {
                Ok(RecentReferrer {
                    domain: row.get(0)?,
                    seconds_ago: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(RealtimeStats {
            active_sessions,
            pageviews_30min,
            current_pages,
            recent_referrers,
        })
    }

    pub fn get_content_stats(&self, content_id: i64) -> Result<ContentStats> {
        let conn = self.db.get()?;

        // Try to get from analytics_content first
        let cached: Option<(i64, i64, Option<String>, Option<String>, String, Option<f64>)> = conn
            .query_row(
                r#"
                SELECT total_pageviews, unique_sessions, first_viewed_at, last_viewed_at,
                       COALESCE(top_referrers, '[]'), bounce_rate
                FROM analytics_content
                WHERE content_id = ?1
                "#,
                [content_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .ok();

        if let Some((total_pageviews, unique_sessions, first_viewed_at, last_viewed_at, referrers_json, bounce_rate)) = cached {
            // Get view trend from events
            let mut stmt = conn.prepare(
                r#"
                SELECT date(timestamp) as day, COUNT(*) as views
                FROM analytics_events
                WHERE content_id = ?1 AND timestamp >= datetime('now', '-30 days')
                GROUP BY day
                ORDER BY day ASC
                "#,
            )?;
            let view_trend: Vec<i64> = stmt
                .query_map([content_id], |row| row.get::<_, i64>(1))?
                .filter_map(|r| r.ok())
                .collect();

            let referrer_data: Vec<(String, i64)> = serde_json::from_str(&referrers_json).unwrap_or_default();
            let total_ref: i64 = referrer_data.iter().map(|(_, c)| c).sum();
            let top_referrers: Vec<ReferrerStats> = referrer_data
                .into_iter()
                .map(|(domain, sessions)| ReferrerStats {
                    domain,
                    sessions,
                    percentage: if total_ref > 0 {
                        (sessions as f64 / total_ref as f64) * 100.0
                    } else {
                        0.0
                    },
                })
                .collect();

            return Ok(ContentStats {
                content_id,
                total_pageviews,
                unique_sessions,
                first_viewed_at,
                last_viewed_at,
                view_trend,
                top_referrers,
                bounce_rate: bounce_rate.unwrap_or(0.0),
            });
        }

        // Fall back to querying events directly
        let (total_pageviews, unique_sessions): (i64, i64) = conn
            .query_row(
                r#"
                SELECT COUNT(*), COUNT(DISTINCT session_hash)
                FROM analytics_events
                WHERE content_id = ?1
                "#,
                [content_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0, 0));

        let first_viewed_at: Option<String> = conn
            .query_row(
                "SELECT MIN(timestamp) FROM analytics_events WHERE content_id = ?1",
                [content_id],
                |row| row.get(0),
            )
            .ok();

        let last_viewed_at: Option<String> = conn
            .query_row(
                "SELECT MAX(timestamp) FROM analytics_events WHERE content_id = ?1",
                [content_id],
                |row| row.get(0),
            )
            .ok();

        let mut stmt = conn.prepare(
            r#"
            SELECT date(timestamp) as day, COUNT(*) as views
            FROM analytics_events
            WHERE content_id = ?1 AND timestamp >= datetime('now', '-30 days')
            GROUP BY day
            ORDER BY day ASC
            "#,
        )?;
        let view_trend: Vec<i64> = stmt
            .query_map([content_id], |row| row.get::<_, i64>(1))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ContentStats {
            content_id,
            total_pageviews,
            unique_sessions,
            first_viewed_at,
            last_viewed_at,
            view_trend,
            top_referrers: vec![],
            bounce_rate: 0.0,
        })
    }

    /// Get content performance data for dashboard
    pub fn get_content_performance(&self, days: i64, limit: i64) -> Result<Vec<ContentPerformance>> {
        let conn = self.db.get()?;

        let cutoff = (chrono::Utc::now() - chrono::TimeDelta::days(days))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let prev_cutoff = (chrono::Utc::now() - chrono::TimeDelta::days(days * 2))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                c.id,
                c.title,
                c.content_type,
                c.slug,
                COUNT(e.id) as pageviews,
                COUNT(DISTINCT e.session_hash) as sessions,
                COALESCE(ac.bounce_rate, 0) as bounce_rate
            FROM content c
            LEFT JOIN analytics_events e ON e.content_id = c.id AND e.timestamp >= ?1
            LEFT JOIN analytics_content ac ON ac.content_id = c.id
            WHERE c.status = 'published'
            GROUP BY c.id
            HAVING pageviews > 0
            ORDER BY pageviews DESC
            LIMIT ?2
            "#,
        )?;

        let mut results: Vec<ContentPerformance> = stmt
            .query_map(rusqlite::params![&cutoff, limit], |row| {
                Ok(ContentPerformance {
                    content_id: row.get(0)?,
                    title: row.get(1)?,
                    content_type: row.get(2)?,
                    slug: row.get(3)?,
                    pageviews: row.get(4)?,
                    unique_sessions: row.get(5)?,
                    avg_time_seconds: 0, // Will be calculated if we have session data
                    bounce_rate: row.get(6)?,
                    trend: "stable".to_string(),
                    trend_percent: 0.0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Calculate trends by comparing to previous period
        for content in &mut results {
            let prev_views: i64 = conn
                .query_row(
                    r#"
                    SELECT COUNT(*)
                    FROM analytics_events
                    WHERE content_id = ?1 AND timestamp >= ?2 AND timestamp < ?3
                    "#,
                    rusqlite::params![content.content_id, &prev_cutoff, &cutoff],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if prev_views > 0 {
                let change = ((content.pageviews - prev_views) as f64 / prev_views as f64) * 100.0;
                content.trend_percent = change;
                content.trend = if change > 10.0 {
                    "up".to_string()
                } else if change < -10.0 {
                    "down".to_string()
                } else {
                    "stable".to_string()
                };
            }
        }

        Ok(results)
    }

    pub fn aggregate_hourly(&self) -> Result<usize> {
        let conn = self.db.get()?;

        let count = conn.execute(
            r#"
            INSERT INTO analytics_hourly (hour, path, content_id, content_type,
                                          pageviews, unique_sessions, avg_response_time_ms, error_count)
            SELECT
                strftime('%Y-%m-%dT%H:00:00Z', timestamp) as hour,
                path,
                content_id,
                content_type,
                COUNT(*) as pageviews,
                COUNT(DISTINCT session_hash) as unique_sessions,
                AVG(response_time_ms) as avg_response_time_ms,
                SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as error_count
            FROM analytics_events
            WHERE timestamp < strftime('%Y-%m-%dT%H:00:00Z', 'now')
              AND timestamp >= datetime('now', '-2 hours')
            GROUP BY hour, path, content_id, content_type
            ON CONFLICT(hour, path) DO UPDATE SET
                pageviews = analytics_hourly.pageviews + excluded.pageviews,
                unique_sessions = analytics_hourly.unique_sessions + excluded.unique_sessions,
                error_count = analytics_hourly.error_count + excluded.error_count
            "#,
            [],
        )?;

        // Clean up old raw events based on config
        let retention_hours = self.config.raw_event_retention_hours as i64;
        conn.execute(
            &format!(
                "DELETE FROM analytics_events WHERE timestamp < datetime('now', '-{} hours')",
                retention_hours
            ),
            [],
        )?;

        Ok(count)
    }

    /// Aggregate daily statistics
    pub fn aggregate_daily(&self) -> Result<usize> {
        let conn = self.db.get()?;

        let yesterday = (chrono::Utc::now() - chrono::TimeDelta::days(1))
            .format("%Y-%m-%d")
            .to_string();

        // Check if already aggregated
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM analytics_daily WHERE date = ?1)",
                [&yesterday],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Ok(0);
        }

        // Compute daily stats from hourly data
        let start = format!("{}T00:00:00Z", yesterday);
        let end = format!("{}T23:59:59Z", yesterday);

        let (total_pageviews, unique_sessions, avg_response, error_count): (i64, i64, Option<f64>, i64) = conn
            .query_row(
                r#"
                SELECT
                    COALESCE(SUM(pageviews), 0),
                    COALESCE(SUM(unique_sessions), 0),
                    AVG(avg_response_time_ms),
                    COALESCE(SUM(error_count), 0)
                FROM analytics_hourly
                WHERE hour >= ?1 AND hour <= ?2
                "#,
                [&start, &end],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or((0, 0, None, 0));

        // Top pages
        let mut stmt = conn.prepare(
            r#"
            SELECT path, SUM(pageviews) as views, SUM(unique_sessions) as sessions
            FROM analytics_hourly
            WHERE hour >= ?1 AND hour <= ?2
            GROUP BY path
            ORDER BY views DESC
            LIMIT 10
            "#,
        )?;
        let top_pages: Vec<PageStats> = stmt
            .query_map([&start, &end], |row| {
                Ok(PageStats {
                    path: row.get(0)?,
                    title: None,
                    content_type: None,
                    pageviews: row.get(1)?,
                    unique_sessions: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Top posts (filter by content_type)
        let mut stmt = conn.prepare(
            r#"
            SELECT path, SUM(pageviews) as views, SUM(unique_sessions) as sessions
            FROM analytics_hourly
            WHERE hour >= ?1 AND hour <= ?2 AND content_type = 'post'
            GROUP BY path
            ORDER BY views DESC
            LIMIT 10
            "#,
        )?;
        let top_posts: Vec<PageStats> = stmt
            .query_map([&start, &end], |row| {
                Ok(PageStats {
                    path: row.get(0)?,
                    title: None,
                    content_type: Some("post".to_string()),
                    pageviews: row.get(1)?,
                    unique_sessions: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Get referrers, countries, devices, browsers from events (if still available)
        let mut referrers: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT referrer_domain, COUNT(*) as count
            FROM analytics_events
            WHERE timestamp >= ?1 AND timestamp <= ?2 AND referrer_domain IS NOT NULL
            GROUP BY referrer_domain
            "#,
        )?;
        for row in stmt.query_map([&start, &end], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            if let Ok((domain, count)) = row {
                referrers.insert(domain, count);
            }
        }

        let mut countries: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT country_code, COUNT(*) as count
            FROM analytics_events
            WHERE timestamp >= ?1 AND timestamp <= ?2 AND country_code IS NOT NULL
            GROUP BY country_code
            "#,
        )?;
        for row in stmt.query_map([&start, &end], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            if let Ok((code, count)) = row {
                countries.insert(code, count);
            }
        }

        let mut devices: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT device_type, COUNT(*) as count
            FROM analytics_events
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY device_type
            "#,
        )?;
        for row in stmt.query_map([&start, &end], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            if let Ok((device, count)) = row {
                devices.insert(device, count);
            }
        }

        let mut browsers: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT browser_family, COUNT(*) as count
            FROM analytics_events
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY browser_family
            "#,
        )?;
        for row in stmt.query_map([&start, &end], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })? {
            if let Ok((browser, count)) = row {
                browsers.insert(browser, count);
            }
        }

        // Count views of new content (< 7 days old)
        let new_content_views: i64 = conn
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM analytics_events e
                JOIN content c ON e.content_id = c.id
                WHERE e.timestamp >= ?1 AND e.timestamp <= ?2
                  AND c.created_at >= datetime('now', '-7 days')
                "#,
                [&start, &end],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let error_rate = if total_pageviews > 0 {
            (error_count as f64 / total_pageviews as f64) * 100.0
        } else {
            0.0
        };

        // Insert daily record
        conn.execute(
            r#"
            INSERT INTO analytics_daily (date, total_pageviews, unique_sessions, top_pages, top_posts,
                                         referrers, countries, devices, browsers, avg_response_time_ms,
                                         error_rate, new_content_views)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            rusqlite::params![
                yesterday,
                total_pageviews,
                unique_sessions,
                serde_json::to_string(&top_pages)?,
                serde_json::to_string(&top_posts)?,
                serde_json::to_string(&referrers)?,
                serde_json::to_string(&countries)?,
                serde_json::to_string(&devices)?,
                serde_json::to_string(&browsers)?,
                avg_response.map(|v| v as i64),
                error_rate,
                new_content_views,
            ],
        )?;

        // Update content analytics bounce rates
        self.update_bounce_rates()?;

        // Clean up old hourly data based on config
        if self.config.hourly_retention_days > 0 {
            conn.execute(
                &format!(
                    "DELETE FROM analytics_hourly WHERE hour < datetime('now', '-{} days')",
                    self.config.hourly_retention_days
                ),
                [],
            )?;
        }

        // Clean up old daily data based on config
        if self.config.daily_retention_days > 0 {
            conn.execute(
                &format!(
                    "DELETE FROM analytics_daily WHERE date < date('now', '-{} days')",
                    self.config.daily_retention_days
                ),
                [],
            )?;
        }

        Ok(1)
    }

    /// Update bounce rates for all content in analytics_content
    fn update_bounce_rates(&self) -> Result<()> {
        let conn = self.db.get()?;

        // Calculate bounce rate: single-page sessions / total sessions
        conn.execute(
            r#"
            UPDATE analytics_content
            SET bounce_rate = (
                SELECT CAST(
                    SUM(CASE WHEN session_pageviews = 1 THEN 1 ELSE 0 END) AS REAL
                ) / NULLIF(COUNT(DISTINCT session_hash), 0) * 100
                FROM (
                    SELECT session_hash, COUNT(*) as session_pageviews
                    FROM analytics_events
                    WHERE content_id = analytics_content.content_id
                    GROUP BY session_hash
                )
            )
            WHERE content_id IN (SELECT DISTINCT content_id FROM analytics_events WHERE content_id IS NOT NULL)
            "#,
            [],
        )?;

        Ok(())
    }

    pub fn cleanup_old_data(&self, hourly_retention_days: i64) -> Result<()> {
        let conn = self.db.get()?;

        conn.execute(
            "DELETE FROM analytics_hourly WHERE hour < datetime('now', ?1)",
            [format!("-{} days", hourly_retention_days)],
        )?;

        Ok(())
    }

    /// Export analytics data
    pub fn export(&self, days: i64, format: ExportFormat) -> Result<String> {
        let now = chrono::Utc::now();
        let start_date = (now - chrono::TimeDelta::days(days))
            .format("%Y-%m-%d")
            .to_string();
        let end_date = now.format("%Y-%m-%d").to_string();

        let summary = self.get_summary(days)?;

        let conn = self.db.get()?;

        // Get daily stats
        let mut stmt = conn.prepare(
            r#"
            SELECT date, total_pageviews, unique_sessions, top_pages, top_posts,
                   referrers, countries, devices, browsers, avg_response_time_ms,
                   error_rate, new_content_views
            FROM analytics_daily
            WHERE date >= ?1 AND date <= ?2
            ORDER BY date ASC
            "#,
        )?;

        let daily_stats: Vec<DailyStats> = stmt
            .query_map([&start_date, &end_date], |row| {
                let top_pages_json: String = row.get(3)?;
                let top_posts_json: String = row.get(4)?;
                let referrers_json: String = row.get(5)?;
                let countries_json: String = row.get(6)?;
                let devices_json: String = row.get(7)?;
                let browsers_json: String = row.get(8)?;

                Ok(DailyStats {
                    date: row.get(0)?,
                    total_pageviews: row.get(1)?,
                    unique_sessions: row.get(2)?,
                    top_pages: serde_json::from_str(&top_pages_json).unwrap_or_default(),
                    top_posts: serde_json::from_str(&top_posts_json).unwrap_or_default(),
                    referrers: serde_json::from_str(&referrers_json).unwrap_or_default(),
                    countries: serde_json::from_str(&countries_json).unwrap_or_default(),
                    devices: serde_json::from_str(&devices_json).unwrap_or_default(),
                    browsers: serde_json::from_str(&browsers_json).unwrap_or_default(),
                    avg_response_time_ms: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
                    error_rate: row.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
                    new_content_views: row.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let export = AnalyticsExport {
            exported_at: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            date_range: DateRange {
                start: start_date,
                end: end_date,
            },
            summary: ExportSummary {
                total_pageviews: summary.total_pageviews,
                unique_sessions: summary.unique_sessions,
                avg_response_time_ms: summary.avg_response_time_ms,
            },
            daily_stats,
            top_pages: summary.top_pages,
            referrers: summary.top_referrers,
            countries: summary.countries,
        };

        match format {
            ExportFormat::Json => Ok(serde_json::to_string_pretty(&export)?),
            ExportFormat::Csv => {
                let mut csv = String::new();
                csv.push_str("date,pageviews,sessions,avg_response_ms,error_rate\n");
                for day in &export.daily_stats {
                    csv.push_str(&format!(
                        "{},{},{},{},{:.2}\n",
                        day.date,
                        day.total_pageviews,
                        day.unique_sessions,
                        day.avg_response_time_ms,
                        day.error_rate
                    ));
                }
                Ok(csv)
            }
        }
    }
}

pub fn generate_session_hash(ip: &str, user_agent: &str, daily_salt: &str) -> String {
    let anonymized_ip = anonymize_ip(ip);
    let browser = extract_browser_family(user_agent);

    let input = format!("{}|{}|{}", daily_salt, anonymized_ip, browser);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

pub fn get_daily_salt(db: &Database) -> Result<String> {
    let conn = db.get()?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let key = format!("session_salt_{}", today);

    let existing: Option<String> = conn
        .query_row(
            "SELECT value FROM analytics_settings WHERE key = ?1",
            [&key],
            |row| row.get(0),
        )
        .ok();

    if let Some(salt) = existing {
        return Ok(salt);
    }

    let salt: String = (0..32)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect();

    conn.execute(
        "INSERT OR REPLACE INTO analytics_settings (key, value) VALUES (?1, ?2)",
        [&key, &salt],
    )?;

    conn.execute(
        "DELETE FROM analytics_settings WHERE key LIKE 'session_salt_%' AND key != ?1",
        [&key],
    )?;

    Ok(salt)
}

fn anonymize_ip(ip: &str) -> String {
    if ip.contains(':') {
        // IPv6: keep first 3 segments
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.len() >= 4 {
            return format!("{}:{}:{}:*", parts[0], parts[1], parts[2]);
        }
    } else {
        // IPv4: zero last two octets
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            return format!("{}.{}.0.0", parts[0], parts[1]);
        }
    }
    "unknown".to_string()
}

pub fn extract_browser_family(user_agent: &str) -> String {
    let ua = user_agent.to_lowercase();
    if ua.contains("firefox") {
        "Firefox".to_string()
    } else if ua.contains("edg/") || ua.contains("edge") {
        "Edge".to_string()
    } else if ua.contains("chrome") || ua.contains("chromium") {
        "Chrome".to_string()
    } else if ua.contains("safari") {
        "Safari".to_string()
    } else if ua.contains("opera") || ua.contains("opr/") {
        "Opera".to_string()
    } else {
        "Other".to_string()
    }
}

pub fn extract_device_type(user_agent: &str) -> DeviceType {
    let ua = user_agent.to_lowercase();
    if ua.contains("mobile") || ua.contains("android") && !ua.contains("tablet") {
        DeviceType::Mobile
    } else if ua.contains("tablet") || ua.contains("ipad") {
        DeviceType::Tablet
    } else {
        DeviceType::Desktop
    }
}

pub fn extract_referrer_domain(referrer: &str) -> Option<String> {
    if referrer.is_empty() {
        return None;
    }
    url::Url::parse(referrer)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Lookup country code from IP address using embedded database
/// This uses a simplified approach based on IP ranges for major regions
pub fn lookup_country(ip: &str) -> Option<String> {
    // Parse IPv4 address
    let parts: Vec<u8> = ip
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    if parts.len() != 4 {
        return None;
    }

    let first_octet = parts[0];
    let second_octet = parts[1];

    // Check private/local ranges first - no country
    if first_octet == 10
        || first_octet == 127
        || (first_octet == 172 && (16..=31).contains(&second_octet))
        || (first_octet == 192 && second_octet == 168)
        || first_octet == 169
    {
        return None;
    }

    // Simplified country lookup based on common IP allocations
    // This is a rough approximation - for production, use MaxMind GeoLite2
    match first_octet {
        // Europe - RIPE allocations
        77..=95 | 145..=151 | 176..=185 | 193..=195 => {
            match second_octet {
                0..=50 => Some("DE".to_string()),
                51..=100 => Some("GB".to_string()),
                101..=150 => Some("FR".to_string()),
                151..=200 => Some("NL".to_string()),
                _ => Some("EU".to_string()),
            }
        }

        // Asia-Pacific - APNIC allocations
        1 | 2 | 27 | 36..=39 | 42..=49 | 58..=61 | 101..=126 | 202..=223 => {
            match second_octet {
                0..=50 => Some("JP".to_string()),
                51..=100 => Some("CN".to_string()),
                101..=150 => Some("AU".to_string()),
                151..=200 => Some("IN".to_string()),
                _ => Some("AP".to_string()),
            }
        }

        // North America (US/CA) - ARIN allocations
        3..=26 | 28..=35 | 40 | 41 | 50..=57 | 63..=76 | 96..=100 | 128..=144 |
        152..=175 | 186..=191 | 196..=201 => {
            if second_octet < 128 {
                Some("US".to_string())
            } else {
                Some("CA".to_string())
            }
        }

        _ => None,
    }
}

fn country_name(code: &str) -> String {
    match code {
        "US" => "United States",
        "GB" => "United Kingdom",
        "DE" => "Germany",
        "FR" => "France",
        "CA" => "Canada",
        "AU" => "Australia",
        "JP" => "Japan",
        "CN" => "China",
        "IN" => "India",
        "BR" => "Brazil",
        "NL" => "Netherlands",
        "ES" => "Spain",
        "IT" => "Italy",
        "SE" => "Sweden",
        "NO" => "Norway",
        "DK" => "Denmark",
        "FI" => "Finland",
        "PL" => "Poland",
        "RU" => "Russia",
        "KR" => "South Korea",
        "MX" => "Mexico",
        "AR" => "Argentina",
        "ZA" => "South Africa",
        "NG" => "Nigeria",
        "EG" => "Egypt",
        "SG" => "Singapore",
        "HK" => "Hong Kong",
        "TW" => "Taiwan",
        "NZ" => "New Zealand",
        "IE" => "Ireland",
        "CH" => "Switzerland",
        "AT" => "Austria",
        "BE" => "Belgium",
        "PT" => "Portugal",
        "EU" => "Europe",
        "AP" => "Asia-Pacific",
        _ => code,
    }
    .to_string()
}

/// Run hourly and daily aggregation jobs
pub async fn run_aggregation_job(analytics: Arc<Analytics>) {
    let mut hourly_interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // 1 hour
    let mut daily_check = tokio::time::interval(std::time::Duration::from_secs(3600 * 6)); // Check every 6 hours

    loop {
        tokio::select! {
            _ = hourly_interval.tick() => {
                // Hourly aggregation
                match analytics.aggregate_hourly() {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!("Analytics: aggregated {} hourly records", count);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Analytics hourly aggregation failed: {}", e);
                    }
                }

                if let Err(e) = analytics.cleanup_old_data(90) {
                    tracing::error!("Analytics cleanup failed: {}", e);
                }
            }
            _ = daily_check.tick() => {
                // Check if daily aggregation is needed (runs once per day)
                let now = chrono::Utc::now();
                // Only run daily aggregation after midnight (0-6 AM UTC)
                if now.hour() < 6 {
                    match analytics.aggregate_daily() {
                        Ok(count) => {
                            if count > 0 {
                                tracing::info!("Analytics: completed daily aggregation");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Analytics daily aggregation failed: {}", e);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_config_should_track() {
        let config = AnalyticsConfig::default();

        assert!(config.should_track("/posts/hello"));
        assert!(config.should_track("/"));
        assert!(!config.should_track("/admin"));
        assert!(!config.should_track("/admin/posts"));
        assert!(!config.should_track("/static/style.css"));
        assert!(!config.should_track("/media/image.jpg"));
        assert!(!config.should_track("/robots.txt"));
    }

    #[test]
    fn test_analytics_config_dnt() {
        let config = AnalyticsConfig::default();

        assert!(config.should_respect_dnt(Some("1")));
        assert!(!config.should_respect_dnt(Some("0")));
        assert!(!config.should_respect_dnt(None));

        let config_no_dnt = AnalyticsConfig {
            respect_dnt: false,
            ..Default::default()
        };
        assert!(!config_no_dnt.should_respect_dnt(Some("1")));
    }

    #[test]
    fn test_lookup_country() {
        // US IPs
        assert_eq!(lookup_country("8.8.8.8"), Some("US".to_string()));
        assert_eq!(lookup_country("4.4.4.4"), Some("US".to_string()));

        // Europe IPs
        assert_eq!(lookup_country("80.80.80.80"), Some("GB".to_string()));

        // Private IPs should return None
        assert_eq!(lookup_country("192.168.1.1"), None);
        assert_eq!(lookup_country("10.0.0.1"), None);
        assert_eq!(lookup_country("127.0.0.1"), None);
        assert_eq!(lookup_country("172.16.0.1"), None);
        assert_eq!(lookup_country("172.31.255.255"), None);

        // Invalid IPs
        assert_eq!(lookup_country("invalid"), None);
        assert_eq!(lookup_country("256.1.1.1"), None);
    }

    #[test]
    fn test_anonymize_ip() {
        assert_eq!(anonymize_ip("192.168.1.100"), "192.168.0.0");
        assert_eq!(anonymize_ip("10.20.30.40"), "10.20.0.0");
        assert_eq!(anonymize_ip("2001:db8:85a3:8d3:1319:8a2e:370:7348"), "2001:db8:85a3:*");
        assert_eq!(anonymize_ip("invalid"), "unknown");
    }

    #[test]
    fn test_extract_browser_family() {
        assert_eq!(
            extract_browser_family("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/91.0"),
            "Chrome"
        );
        assert_eq!(
            extract_browser_family("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0"),
            "Firefox"
        );
        assert_eq!(
            extract_browser_family("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15"),
            "Safari"
        );
    }

    #[test]
    fn test_extract_device_type() {
        assert!(matches!(
            extract_device_type("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
            DeviceType::Desktop
        ));
        assert!(matches!(
            extract_device_type("Mozilla/5.0 (iPhone; CPU iPhone OS 14_6 like Mac OS X) Mobile"),
            DeviceType::Mobile
        ));
        assert!(matches!(
            extract_device_type("Mozilla/5.0 (iPad; CPU OS 14_6 like Mac OS X)"),
            DeviceType::Tablet
        ));
    }
}
