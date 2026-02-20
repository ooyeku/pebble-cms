use clap::Parser;
use pebble_cms::cli::{Cli, Commands};
use pebble_cms::global::PebbleHome;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pebble_cms=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _ = PebbleHome::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { path, name }) => {
            pebble_cms::cli::init::run(path, name).await?;
        }
        Some(Commands::Serve { host, port }) => {
            pebble_cms::cli::serve::run(&cli.config, &host, port).await?;
        }
        Some(Commands::Deploy { host, port }) => {
            pebble_cms::cli::deploy::run(&cli.config, &host, port).await?;
        }
        Some(Commands::Build { output, base_url }) => {
            pebble_cms::cli::build::run(&cli.config, &output, base_url).await?;
        }
        Some(Commands::Export {
            output,
            include_drafts,
            include_media,
            format,
        }) => {
            pebble_cms::cli::export::run(&cli.config, &output, include_drafts, include_media, &format).await?;
        }
        Some(Commands::Import { path, overwrite }) => {
            pebble_cms::cli::import::run(&cli.config, &path, overwrite).await?;
        }
        Some(Commands::ImportWp { file, overwrite }) => {
            pebble_cms::cli::import_wordpress::run(&cli.config, &file, overwrite).await?;
        }
        Some(Commands::ImportGhost { file, overwrite }) => {
            pebble_cms::cli::import_ghost::run(&cli.config, &file, overwrite).await?;
        }
        Some(Commands::Backup { command }) => {
            pebble_cms::cli::backup::run(&cli.config, command).await?;
        }
        Some(Commands::Migrate { command }) => {
            pebble_cms::cli::migrate::run(&cli.config, command).await?;
        }
        Some(Commands::Doctor) => {
            pebble_cms::cli::doctor::run(&cli.config).await?;
        }
        Some(Commands::Rerender) => {
            pebble_cms::cli::rerender::run(&cli.config).await?;
        }
        Some(Commands::User { command }) => {
            pebble_cms::cli::user::run(&cli.config, command).await?;
        }
        Some(Commands::Config { command }) => {
            pebble_cms::cli::config::run(command).await?;
        }
        Some(Commands::Registry { command }) => {
            pebble_cms::cli::registry::run(command).await?;
        }
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
