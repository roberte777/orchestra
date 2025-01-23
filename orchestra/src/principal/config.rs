use serde::{Deserialize, Serialize};

fn default_heartbeat_interval() -> u64 {
    500
}

#[derive(Serialize, Deserialize)]
pub struct PrincipalConfig {
    pub maestro_server_address: String,
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_ms: u64,
}
