use clap::Parser;
use orchestra::{
    cli::{Commands, OrchestraArgs},
    maestro::run_maestro,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = OrchestraArgs::parse();
    match args.command {
        Commands::Maestro(maestro_args) => {
            run_maestro(maestro_args).await?;
        }
    }
    Ok(())
}
