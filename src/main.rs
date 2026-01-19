use clap::Parser;
use pebble::cli::{Cli, Commands};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pebble=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { path, name }) => {
            pebble::cli::init::run(path, name).await?;
        }
        Some(Commands::Serve { host, port }) => {
            pebble::cli::serve::run(&cli.config, &host, port).await?;
        }
        Some(Commands::Deploy { host, port }) => {
            pebble::cli::deploy::run(&cli.config, &host, port).await?;
        }
        Some(Commands::Export {
            output,
            include_drafts,
        }) => {
            pebble::cli::export::run(&cli.config, &output, include_drafts).await?;
        }
        Some(Commands::Migrate) => {
            pebble::cli::migrate::run(&cli.config).await?;
        }
        Some(Commands::User { command }) => {
            pebble::cli::user::run(&cli.config, command).await?;
        }
        None => {
            // No subcommand provided, print help
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
