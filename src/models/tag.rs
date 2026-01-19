use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagWithCount {
    #[serde(flatten)]
    pub tag: Tag,
    pub count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTag {
    pub name: String,
    pub slug: Option<String>,
}
