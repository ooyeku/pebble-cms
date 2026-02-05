use crate::db::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Publish,
    Unpublish,
    Schedule,
    Restore,
    Login,
    LoginFailed,
    Logout,
    UserCreate,
    UserUpdate,
    UserDelete,
    RoleChange,
    PasswordChange,
    Upload,
    MediaDelete,
    TagCreate,
    TagDelete,
    SettingsUpdate,
    Cleanup,
    Export,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Publish => "publish",
            Self::Unpublish => "unpublish",
            Self::Schedule => "schedule",
            Self::Restore => "restore",
            Self::Login => "login",
            Self::LoginFailed => "login_failed",
            Self::Logout => "logout",
            Self::UserCreate => "user_create",
            Self::UserUpdate => "user_update",
            Self::UserDelete => "user_delete",
            Self::RoleChange => "role_change",
            Self::PasswordChange => "password_change",
            Self::Upload => "upload",
            Self::MediaDelete => "media_delete",
            Self::TagCreate => "tag_create",
            Self::TagDelete => "tag_delete",
            Self::SettingsUpdate => "settings_update",
            Self::Cleanup => "cleanup",
            Self::Export => "export",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "create" => Some(Self::Create),
            "update" => Some(Self::Update),
            "delete" => Some(Self::Delete),
            "publish" => Some(Self::Publish),
            "unpublish" => Some(Self::Unpublish),
            "schedule" => Some(Self::Schedule),
            "restore" => Some(Self::Restore),
            "login" => Some(Self::Login),
            "login_failed" => Some(Self::LoginFailed),
            "logout" => Some(Self::Logout),
            "user_create" => Some(Self::UserCreate),
            "user_update" => Some(Self::UserUpdate),
            "user_delete" => Some(Self::UserDelete),
            "role_change" => Some(Self::RoleChange),
            "password_change" => Some(Self::PasswordChange),
            "upload" => Some(Self::Upload),
            "media_delete" => Some(Self::MediaDelete),
            "tag_create" => Some(Self::TagCreate),
            "tag_delete" => Some(Self::TagDelete),
            "settings_update" => Some(Self::SettingsUpdate),
            "cleanup" => Some(Self::Cleanup),
            "export" => Some(Self::Export),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Update => "Update",
            Self::Delete => "Delete",
            Self::Publish => "Publish",
            Self::Unpublish => "Unpublish",
            Self::Schedule => "Schedule",
            Self::Restore => "Restore",
            Self::Login => "Login",
            Self::LoginFailed => "Login Failed",
            Self::Logout => "Logout",
            Self::UserCreate => "User Create",
            Self::UserUpdate => "User Update",
            Self::UserDelete => "User Delete",
            Self::RoleChange => "Role Change",
            Self::PasswordChange => "Password Change",
            Self::Upload => "Upload",
            Self::MediaDelete => "Media Delete",
            Self::TagCreate => "Tag Create",
            Self::TagDelete => "Tag Delete",
            Self::SettingsUpdate => "Settings Update",
            Self::Cleanup => "Cleanup",
            Self::Export => "Export",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Content,
    Auth,
    User,
    Media,
    Tag,
    Settings,
    System,
}

impl AuditCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Auth => "auth",
            Self::User => "user",
            Self::Media => "media",
            Self::Tag => "tag",
            Self::Settings => "settings",
            Self::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "content" => Some(Self::Content),
            "auth" => Some(Self::Auth),
            "user" => Some(Self::User),
            "media" => Some(Self::Media),
            "tag" => Some(Self::Tag),
            "settings" => Some(Self::Settings),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Content => "Content",
            Self::Auth => "Authentication",
            Self::User => "User Management",
            Self::Media => "Media",
            Self::Tag => "Tags",
            Self::Settings => "Settings",
            Self::System => "System",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub user_role: Option<String>,
    pub action: String,
    pub category: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub entity_title: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub changes: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
}

impl AuditEntry {
    pub fn action_enum(&self) -> Option<AuditAction> {
        AuditAction::from_str(&self.action)
    }

    pub fn category_enum(&self) -> Option<AuditCategory> {
        AuditCategory::from_str(&self.category)
    }

    pub fn is_failure(&self) -> bool {
        self.status == "failure"
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub user_role: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl AuditContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user(mut self, id: i64, username: &str, role: &str) -> Self {
        self.user_id = Some(id);
        self.username = Some(username.to_string());
        self.user_role = Some(role.to_string());
        self
    }

    pub fn with_request(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.ip_address = ip;
        self.user_agent = user_agent;
        self
    }
}

#[derive(Debug, Clone)]
pub struct AuditLogBuilder {
    action: AuditAction,
    category: AuditCategory,
    entity_type: Option<String>,
    entity_id: Option<i64>,
    entity_title: Option<String>,
    changes: Option<serde_json::Value>,
    metadata: serde_json::Value,
    status: String,
    error_message: Option<String>,
}

impl AuditLogBuilder {
    pub fn new(action: AuditAction, category: AuditCategory) -> Self {
        Self {
            action,
            category,
            entity_type: None,
            entity_id: None,
            entity_title: None,
            changes: None,
            metadata: json!({}),
            status: "success".to_string(),
            error_message: None,
        }
    }

    pub fn entity(mut self, entity_type: &str, id: i64, title: Option<&str>) -> Self {
        self.entity_type = Some(entity_type.to_string());
        self.entity_id = Some(id);
        self.entity_title = title.map(|s| s.to_string());
        self
    }

    pub fn entity_type_only(mut self, entity_type: &str) -> Self {
        self.entity_type = Some(entity_type.to_string());
        self
    }

    pub fn changes(mut self, changes: serde_json::Value) -> Self {
        self.changes = Some(changes);
        self
    }

    pub fn metadata_value(mut self, key: &str, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.to_string(), value);
        }
        self
    }

    pub fn failure(mut self, error: &str) -> Self {
        self.status = "failure".to_string();
        self.error_message = Some(error.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditSummary {
    pub total_events: i64,
    pub events_today: i64,
    pub failed_events: i64,
    pub active_users_today: i64,
    pub recent_failures: Vec<AuditEntry>,
    pub actions_breakdown: Vec<ActionCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionCount {
    pub action: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub action: Option<String>,
    pub category: Option<String>,
    pub entity_type: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

pub fn log(db: &Database, ctx: &AuditContext, builder: AuditLogBuilder) -> Result<i64> {
    let conn = db.get()?;

    let changes_json = builder.changes.map(|c| c.to_string());
    let metadata_json = builder.metadata.to_string();

    conn.execute(
        r#"
        INSERT INTO audit_logs (
            user_id, username, user_role, action, category,
            entity_type, entity_id, entity_title,
            ip_address, user_agent, status, error_message,
            changes, metadata
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        rusqlite::params![
            ctx.user_id,
            ctx.username,
            ctx.user_role,
            builder.action.as_str(),
            builder.category.as_str(),
            builder.entity_type,
            builder.entity_id,
            builder.entity_title,
            ctx.ip_address,
            ctx.user_agent,
            builder.status,
            builder.error_message,
            changes_json,
            metadata_json,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn list_logs(
    db: &Database,
    filter: &AuditFilter,
    limit: usize,
    offset: usize,
) -> Result<Vec<AuditEntry>> {
    let conn = db.get()?;

    let (where_clause, params) = build_filter_clause(filter);

    let sql = format!(
        r#"
        SELECT id, timestamp, user_id, username, user_role, action, category,
               entity_type, entity_id, entity_title, ip_address, user_agent,
               status, error_message, changes, metadata
        FROM audit_logs
        {}
        ORDER BY timestamp DESC
        LIMIT ?{} OFFSET ?{}
        "#,
        where_clause,
        params.len() + 1,
        params.len() + 2
    );

    let mut stmt = conn.prepare(&sql)?;

    let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params
        .into_iter()
        .map(|s| Box::new(s) as Box<dyn rusqlite::ToSql>)
        .collect();
    all_params.push(Box::new(limit as i64));
    all_params.push(Box::new(offset as i64));

    let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

    let entries = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                user_id: row.get(2)?,
                username: row.get(3)?,
                user_role: row.get(4)?,
                action: row.get(5)?,
                category: row.get(6)?,
                entity_type: row.get(7)?,
                entity_id: row.get(8)?,
                entity_title: row.get(9)?,
                ip_address: row.get(10)?,
                user_agent: row.get(11)?,
                status: row.get(12)?,
                error_message: row.get(13)?,
                changes: row
                    .get::<_, Option<String>>(14)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                metadata: row
                    .get::<_, String>(15)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(json!({})),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

pub fn count_logs(db: &Database, filter: &AuditFilter) -> Result<i64> {
    let conn = db.get()?;

    let (where_clause, params) = build_filter_clause(filter);

    let sql = format!("SELECT COUNT(*) FROM audit_logs {}", where_clause);

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let count: i64 = stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;

    Ok(count)
}

pub fn get_log(db: &Database, id: i64) -> Result<Option<AuditEntry>> {
    let conn = db.get()?;

    let entry = conn.query_row(
        r#"
        SELECT id, timestamp, user_id, username, user_role, action, category,
               entity_type, entity_id, entity_title, ip_address, user_agent,
               status, error_message, changes, metadata
        FROM audit_logs
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                user_id: row.get(2)?,
                username: row.get(3)?,
                user_role: row.get(4)?,
                action: row.get(5)?,
                category: row.get(6)?,
                entity_type: row.get(7)?,
                entity_id: row.get(8)?,
                entity_title: row.get(9)?,
                ip_address: row.get(10)?,
                user_agent: row.get(11)?,
                status: row.get(12)?,
                error_message: row.get(13)?,
                changes: row
                    .get::<_, Option<String>>(14)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                metadata: row
                    .get::<_, String>(15)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(json!({})),
            })
        },
    );

    match entry {
        Ok(e) => Ok(Some(e)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_summary(db: &Database, days: u32) -> Result<AuditSummary> {
    let conn = db.get()?;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S").to_string();

    let today_start = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let total_events: i64 =
        conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |row| row.get(0))?;

    let events_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_logs WHERE timestamp >= ?1",
        [&today_start],
        |row| row.get(0),
    )?;

    let failed_events: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_logs WHERE status = 'failure' AND timestamp >= ?1",
        [&cutoff_str],
        |row| row.get(0),
    )?;

    let active_users_today: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT user_id) FROM audit_logs WHERE user_id IS NOT NULL AND timestamp >= ?1",
        [&today_start],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        r#"
        SELECT id, timestamp, user_id, username, user_role, action, category,
               entity_type, entity_id, entity_title, ip_address, user_agent,
               status, error_message, changes, metadata
        FROM audit_logs
        WHERE status = 'failure'
        ORDER BY timestamp DESC
        LIMIT 5
        "#,
    )?;

    let recent_failures = stmt
        .query_map([], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                user_id: row.get(2)?,
                username: row.get(3)?,
                user_role: row.get(4)?,
                action: row.get(5)?,
                category: row.get(6)?,
                entity_type: row.get(7)?,
                entity_id: row.get(8)?,
                entity_title: row.get(9)?,
                ip_address: row.get(10)?,
                user_agent: row.get(11)?,
                status: row.get(12)?,
                error_message: row.get(13)?,
                changes: row
                    .get::<_, Option<String>>(14)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                metadata: row
                    .get::<_, String>(15)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(json!({})),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        r#"
        SELECT action, COUNT(*) as count
        FROM audit_logs
        WHERE timestamp >= ?1
        GROUP BY action
        ORDER BY count DESC
        LIMIT 10
        "#,
    )?;

    let actions_breakdown = stmt
        .query_map([&cutoff_str], |row| {
            Ok(ActionCount {
                action: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AuditSummary {
        total_events,
        events_today,
        failed_events,
        active_users_today,
        recent_failures,
        actions_breakdown,
    })
}

pub fn export_logs(db: &Database, filter: &AuditFilter, format: &str) -> Result<String> {
    let logs = list_logs(db, filter, 10000, 0)?;

    match format {
        "csv" => {
            let mut csv = String::from("timestamp,user,action,category,entity_type,entity_id,entity_title,status,ip_address\n");
            for log in logs {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{}\n",
                    log.timestamp,
                    log.username.unwrap_or_default(),
                    log.action,
                    log.category,
                    log.entity_type.unwrap_or_default(),
                    log.entity_id.map(|i| i.to_string()).unwrap_or_default(),
                    log.entity_title.unwrap_or_default().replace(',', ";"),
                    log.status,
                    log.ip_address.unwrap_or_default(),
                ));
            }
            Ok(csv)
        }
        _ => Ok(serde_json::to_string_pretty(&logs)?),
    }
}

pub fn cleanup_old_logs(db: &Database, retention_days: u32) -> Result<usize> {
    if retention_days == 0 {
        return Ok(0);
    }

    let conn = db.get()?;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S").to_string();

    let deleted = conn.execute("DELETE FROM audit_logs WHERE timestamp < ?1", [&cutoff_str])?;

    if deleted > 0 {
        tracing::info!("Cleaned up {} old audit log entries", deleted);
    }

    Ok(deleted)
}

pub fn get_audit_users(db: &Database) -> Result<Vec<(i64, String)>> {
    let conn = db.get()?;

    let mut stmt = conn.prepare(
        r#"
        SELECT DISTINCT user_id, username
        FROM audit_logs
        WHERE user_id IS NOT NULL AND username IS NOT NULL
        ORDER BY username
        "#,
    )?;

    let users = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(users)
}

fn build_filter_clause(filter: &AuditFilter) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut params = Vec::new();

    if let Some(ref user_id) = filter.user_id {
        params.push(user_id.to_string());
        conditions.push(format!("user_id = ?{}", params.len()));
    }

    if let Some(ref username) = filter.username {
        params.push(username.clone());
        conditions.push(format!("username = ?{}", params.len()));
    }

    if let Some(ref action) = filter.action {
        params.push(action.clone());
        conditions.push(format!("action = ?{}", params.len()));
    }

    if let Some(ref category) = filter.category {
        params.push(category.clone());
        conditions.push(format!("category = ?{}", params.len()));
    }

    if let Some(ref entity_type) = filter.entity_type {
        params.push(entity_type.clone());
        conditions.push(format!("entity_type = ?{}", params.len()));
    }

    if let Some(ref status) = filter.status {
        params.push(status.clone());
        conditions.push(format!("status = ?{}", params.len()));
    }

    if let Some(ref search) = filter.search {
        let search_pattern = format!("%{}%", search);
        params.push(search_pattern.clone());
        params.push(search_pattern.clone());
        params.push(search_pattern);
        conditions.push(format!(
            "(username LIKE ?{} OR entity_title LIKE ?{} OR action LIKE ?{})",
            params.len() - 2,
            params.len() - 1,
            params.len()
        ));
    }

    if let Some(ref from_date) = filter.from_date {
        params.push(from_date.clone());
        conditions.push(format!("timestamp >= ?{}", params.len()));
    }

    if let Some(ref to_date) = filter.to_date {
        params.push(format!("{}T23:59:59", to_date));
        conditions.push(format!("timestamp <= ?{}", params.len()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    (where_clause, params)
}

pub fn get_all_actions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("create", "Create"),
        ("update", "Update"),
        ("delete", "Delete"),
        ("publish", "Publish"),
        ("unpublish", "Unpublish"),
        ("schedule", "Schedule"),
        ("restore", "Restore"),
        ("login", "Login"),
        ("login_failed", "Login Failed"),
        ("logout", "Logout"),
        ("user_create", "User Create"),
        ("user_update", "User Update"),
        ("user_delete", "User Delete"),
        ("role_change", "Role Change"),
        ("password_change", "Password Change"),
        ("upload", "Upload"),
        ("media_delete", "Media Delete"),
        ("tag_create", "Tag Create"),
        ("tag_delete", "Tag Delete"),
        ("settings_update", "Settings Update"),
        ("cleanup", "Cleanup"),
        ("export", "Export"),
    ]
}

pub fn get_all_categories() -> Vec<(&'static str, &'static str)> {
    vec![
        ("content", "Content"),
        ("auth", "Authentication"),
        ("user", "User Management"),
        ("media", "Media"),
        ("tag", "Tags"),
        ("settings", "Settings"),
        ("system", "System"),
    ]
}
