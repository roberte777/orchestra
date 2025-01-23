use clap::{Parser, Subcommand};

use crate::maestro::cli::MaestroArgs;

/// Main program for "orchestra", combining maestro and principal
#[derive(Parser, Debug)]
#[command(name = "Orchestra", version, about, long_about = None)]
pub struct OrchestraArgs {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for the "orchestra" binary
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the maestro command
    Maestro(MaestroArgs),
    // /// Run the principal command
    // Principal(PrincipalArgs),
}
