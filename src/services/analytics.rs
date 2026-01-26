use crate::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

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

#[derive(Debug, Clone, Serialize)]
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
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivePage {
    pub path: String,
    pub visitors: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentStats {
    pub content_id: i64,
    pub total_pageviews: i64,
    pub unique_sessions: i64,
    pub first_viewed_at: Option<String>,
    pub last_viewed_at: Option<String>,
    pub view_trend: Vec<i64>,
}

pub struct Analytics {
    db: Database,
}

impl Analytics {
    pub fn new(db: Database) -> Self {
        Self { db }
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
        Ok(())
    }

    pub fn get_summary(&self, days: i64) -> Result<DashboardSummary> {
        let conn = self.db.get()?;

        // Compute cutoff dates in Rust using ISO 8601 format to match stored timestamps
        let now = chrono::Utc::now();
        let cutoff = (now - chrono::TimeDelta::days(days))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let prev_cutoff = (now - chrono::TimeDelta::days(days * 2))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        tracing::debug!("Analytics query: days={}, cutoff={}", days, cutoff);

        // Query both analytics_events (recent) and analytics_hourly (aggregated) tables
        // and combine results for complete data
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
            .max(1); // Ensure at least 1 to avoid division by zero

        tracing::info!(
            "Pageviews over time: {} data points, max={}",
            pageviews_over_time.len(),
            pageviews_max
        );

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

        Ok(RealtimeStats {
            active_sessions,
            pageviews_30min,
            current_pages,
        })
    }

    pub fn get_content_stats(&self, content_id: i64) -> Result<ContentStats> {
        let conn = self.db.get()?;

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
        })
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

        conn.execute(
            "DELETE FROM analytics_events WHERE timestamp < datetime('now', '-48 hours')",
            [],
        )?;

        Ok(count)
    }

    pub fn cleanup_old_data(&self, hourly_retention_days: i64) -> Result<()> {
        let conn = self.db.get()?;

        conn.execute(
            "DELETE FROM analytics_hourly WHERE hour < datetime('now', ?1)",
            [format!("-{} days", hourly_retention_days)],
        )?;

        Ok(())
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
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.len() >= 4 {
            return format!("{}:{}:{}:*", parts[0], parts[1], parts[2]);
        }
    } else {
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
        _ => code,
    }
    .to_string()
}

pub async fn run_aggregation_job(analytics: Arc<Analytics>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));

    loop {
        interval.tick().await;

        match analytics.aggregate_hourly() {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("Analytics: aggregated {} hourly records", count);
                }
            }
            Err(e) => {
                tracing::error!("Analytics aggregation failed: {}", e);
            }
        }

        if let Err(e) = analytics.cleanup_old_data(90) {
            tracing::error!("Analytics cleanup failed: {}", e);
        }
    }
}
