use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Author,
    Viewer,
}

impl FromStr for UserRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "author" => Ok(Self::Author),
            "viewer" => Ok(Self::Viewer),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Author => write!(f, "author"),
            Self::Viewer => write!(f, "viewer"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
}

impl From<&User> for UserSummary {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
        }
    }
}
