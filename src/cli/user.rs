use crate::{services::auth, Config, Database};
use anyhow::Result;
use std::path::Path;

use super::UserCommand;

pub async fn run(config_path: &Path, command: UserCommand) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    match command {
        UserCommand::Add {
            username,
            email,
            role,
            password,
        } => {
            let password = match password {
                Some(p) => p,
                None => {
                    let p = rpassword::prompt_password("Password: ")?;
                    let p_confirm = rpassword::prompt_password("Confirm password: ")?;
                    if p != p_confirm {
                        anyhow::bail!("Passwords do not match");
                    }
                    p
                }
            };

            let role = role.parse().map_err(|_| anyhow::anyhow!("Invalid role"))?;
            auth::create_user(&db, &username, &email, &password, role)?;
            tracing::info!("User '{}' created", username);
        }
        UserCommand::List => {
            let conn = db.get()?;
            let mut stmt = conn.prepare("SELECT username, email, role FROM users")?;
            let users = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            println!("{:<20} {:<30} {:<10}", "USERNAME", "EMAIL", "ROLE");
            println!("{}", "-".repeat(60));
            for user in users {
                let (username, email, role) = user?;
                println!("{:<20} {:<30} {:<10}", username, email, role);
            }
        }
        UserCommand::Remove { username } => {
            let conn = db.get()?;
            let affected = conn.execute("DELETE FROM users WHERE username = ?", [&username])?;
            if affected > 0 {
                tracing::info!("User '{}' removed", username);
            } else {
                tracing::warn!("User '{}' not found", username);
            }
        }
        UserCommand::Passwd { username } => {
            let password = rpassword::prompt_password("New password: ")?;
            let password_confirm = rpassword::prompt_password("Confirm password: ")?;

            if password != password_confirm {
                anyhow::bail!("Passwords do not match");
            }

            auth::update_password(&db, &username, &password)?;
            tracing::info!("Password updated for '{}'", username);
        }
    }

    Ok(())
}
