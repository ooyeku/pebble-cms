use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesWithItems {
    #[serde(flatten)]
    pub series: Series,
    pub items: Vec<SeriesItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesItem {
    pub id: i64,
    pub content_id: i64,
    pub position: i32,
    pub title: String,
    pub slug: String,
    pub status: String,
}

/// Prev/next navigation context for a post within a series.
#[derive(Debug, Clone, Serialize)]
pub struct SeriesNavigation {
    pub series: Series,
    pub current_position: i32,
    pub total_items: usize,
    pub prev: Option<SeriesNavItem>,
    pub next: Option<SeriesNavItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesNavItem {
    pub title: String,
    pub slug: String,
    pub position: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateSeries {
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeries {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}
