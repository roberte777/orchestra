use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MaestroConfig {
    pub server_address: String,
}
