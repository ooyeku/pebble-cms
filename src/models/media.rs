use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Media {
    pub id: i64,
    pub filename: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub alt_text: String,
    pub uploaded_by: Option<i64>,
    pub created_at: String,
}
