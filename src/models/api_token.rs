use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ApiToken {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub permissions: String,
    pub created_by: Option<i64>,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}
