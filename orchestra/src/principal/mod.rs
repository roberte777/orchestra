pub mod process_exits_actor;
use anyhow::Result;
pub mod note_process;
use note_process::ManagedNote;
use reqwest::Client;
use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{
    sync::Mutex,
    task::JoinHandle,
    time::{self},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::maestro::dto::{HeartbeatDto, HeartbeatNoteDto};

#[derive(Default, Debug, Clone)]
pub struct AppState {
    pub notes: Arc<Mutex<HashMap<String, ManagedNote>>>,
}

struct HeartbeatTaskState {
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

pub struct HeartbeatActor {
    url: String,
    heartbat_state: Option<HeartbeatTaskState>,
}

impl HeartbeatActor {
    pub fn new(url: String) -> Self {
        Self {
            url,
            heartbat_state: None,
        }
    }

    pub fn start(&mut self, state: AppState) {
        let mut interval = time::interval(Duration::from_millis(500));
        let cancel = CancellationToken::new();
        let child = cancel.child_token();
        let url = self.url.clone();

        // TODO: Try heartbeat first to see if the url can be reached

        let jh = tokio::spawn(async move {
            let client = Client::new();
            loop {
                //TODO: Take some sort of action if heartbeat fails.
                {
                    let notes = state.notes.lock().await;
                    let _ = heartbeat_task(&url, &client, &notes).await;
                }
                tokio::select! {
                    _ = interval.tick() => {
                        // continue to next iteration
                    }
                    _ = child.cancelled() => { break }
                }
            }
            info!("Exiting heartbeat task");
        });

        let heartbeat_state = HeartbeatTaskState {
            task: jh,
            cancellation_token: cancel,
        };

        self.heartbat_state = Some(heartbeat_state);
    }

    pub async fn stop(&mut self) {
        if let Some(hb_state) = self.heartbat_state.take() {
            hb_state.cancellation_token.cancel();
            _ = hb_state.task.await;
        }
    }
}
async fn heartbeat_task(
    url: &str,
    client: &Client,
    notes: &HashMap<String, ManagedNote>,
) -> Result<()> {
    let notes = notes
        .iter()
        .map(|n| HeartbeatNoteDto {
            symphony: n.1.note.symphony.clone(),
            name: n.1.note.name.clone(),
            state: n.1.note.state.clone(),
        })
        .collect();
    let heartbeat = HeartbeatDto {
        name: "me".to_string(),
        notes,
    };
    let response = client
        .post(url)
        .json(&heartbeat)
        // .header("Content-Type", "application/json")
        .send()
        .await?;
    debug!("Heartbeat response: {:?}", response);
    Ok(())
}
