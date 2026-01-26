use crate::global::{GlobalConfig, PebbleHome};
use anyhow::Result;

pub async fn run(command: super::ConfigCommand) -> Result<()> {
    let home = PebbleHome::init()?;
    let mut config = GlobalConfig::load(&home.config_path)?;

    match command {
        super::ConfigCommand::Get { key } => match config.get(&key) {
            Some(value) => println!("{}", value),
            None => {
                eprintln!("Unknown config key: {}", key);
                std::process::exit(1);
            }
        },
        super::ConfigCommand::Set { key, value } => {
            config.set(&key, &value)?;
            config.save(&home.config_path)?;
            println!("Set {} = {}", key, value);
        }
        super::ConfigCommand::List => {
            let items = config.list();
            let max_key_len = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            for (key, value) in items {
                println!("{:width$}  {}", key, value, width = max_key_len);
            }
        }
        super::ConfigCommand::Remove { key } => {
            if config.remove(&key)? {
                config.save(&home.config_path)?;
                println!("Removed {}", key);
            } else {
                println!("Key not found: {}", key);
            }
        }
        super::ConfigCommand::Path => {
            println!("{}", home.config_path.display());
        }
    }

    Ok(())
}
