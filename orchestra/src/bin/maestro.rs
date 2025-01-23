use clap::Parser;
use orchestra::maestro::{config::MaestroConfig, run_maestro};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = orchestra::maestro::cli::Args::parse();
    let config = std::fs::read_to_string(args.config)?;
    let config: MaestroConfig = serde_json::from_str(&config)?;
    run_maestro(config).await?;
    Ok(())
}
