use clap::Parser;
use orchestra::maestro::run_maestro;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = orchestra::maestro::cli::MaestroArgs::parse();
    run_maestro(args).await?;
    Ok(())
}
