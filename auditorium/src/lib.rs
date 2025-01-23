use std::sync::Arc;

use reqwest::Client;
use tokio::sync::{broadcast::Sender, Mutex};
use tracked_symphonies::InMemoryAuditoriumStore;

pub mod dto;
pub mod tracked_symphonies;

#[derive(Clone)]
pub struct AppState {
    pub maestro_url: String,
    pub client: Client,
    pub tracked_symphonies: Arc<Mutex<InMemoryAuditoriumStore>>,
    pub symphony_tx: Sender<String>,
}
