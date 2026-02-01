use anyhow::Result;
use std::path::Path;

use crate::services::content::rerender_all_content;
use crate::{Config, Database};

pub async fn run(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    println!("Re-rendering all content...");
    let count = rerender_all_content(&db)?;
    println!("Successfully re-rendered {} content items.", count);

    Ok(())
}
