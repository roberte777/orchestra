use std::path::PathBuf;

use clap::Parser;

/// Returns the default configuration path based on the operating system.
fn default_config_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\Maestro\config.json")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/Maestro/config.json")
    } else {
        // Assume a Unix-like OS for the else case
        PathBuf::from("/etc/maestro/config.toml")
    }
}
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "Maestro", version, about, long_about = None)]
pub struct MaestroArgs {
    /// Path to the maestro config file
    #[arg(short, long, default_value_os_t = default_config_path())]
    pub config: PathBuf,
}
