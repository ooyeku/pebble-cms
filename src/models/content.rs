use super::{Tag, UserSummary};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    #[default]
    Post,
    Page,
    Snippet,
}

impl FromStr for ContentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "post" => Ok(Self::Post),
            "page" => Ok(Self::Page),
            "snippet" => Ok(Self::Snippet),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Post => write!(f, "post"),
            Self::Page => write!(f, "page"),
            Self::Snippet => write!(f, "snippet"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContentStatus {
    #[default]
    Draft,
    Published,
    Archived,
}

impl FromStr for ContentStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ContentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Published => write!(f, "published"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Content {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub content_type: ContentType,
    pub body_markdown: String,
    pub body_html: String,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub status: ContentStatus,
    pub published_at: Option<String>,
    pub author_id: Option<i64>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentWithTags {
    #[serde(flatten)]
    pub content: Content,
    pub tags: Vec<Tag>,
    pub author: Option<UserSummary>,
}

#[derive(Debug, Deserialize)]
pub struct CreateContent {
    pub title: String,
    pub slug: Option<String>,
    #[serde(default)]
    pub content_type: ContentType,
    #[serde(default)]
    pub body_markdown: String,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    #[serde(default)]
    pub status: ContentStatus,
    #[serde(default)]
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateContent {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub body_markdown: Option<String>,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub status: Option<ContentStatus>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentSummary {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub status: ContentStatus,
    pub published_at: Option<String>,
    pub created_at: String,
}
