use anyhow::Result;
use clap::Parser;
use orchestra::principal::run_principal;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = orchestra::principal::cli::PrincipalArgs::parse();
    run_principal(args).await?;
    Ok(())
}
