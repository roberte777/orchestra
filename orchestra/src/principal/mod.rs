pub mod cli;
pub mod config;
pub mod process_exits_actor;
use anyhow::{anyhow, Result};
pub mod note_process;
use cli::PrincipalArgs;
use config::PrincipalConfig;
use note_process::{start_note, stop_note, synchronize_state, ManagedNote};
use process_exits_actor::ProcessExitsActor;
use reqwest::Client;
use reqwest_eventsource::EventSource;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tokio_stream::StreamExt;

use tokio::{
    sync::{mpsc::unbounded_channel, Mutex},
    task::JoinHandle,
    time::{self},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::{
    maestro::dto::{HeartbeatDto, HeartbeatNoteDto},
    Note, NoteState,
};

#[derive(Default, Debug, Clone)]
pub struct AppState {
    pub notes: Arc<Mutex<HashMap<String, ManagedNote>>>,
}

pub async fn run_principal(args: PrincipalArgs) -> anyhow::Result<()> {
    if !args.config.exists() {
        return Err(anyhow!(
            "Config path does not exist. Config path: {}",
            args.config.display()
        ));
    }
    let config_path = if args.config.is_dir() {
        args.config.join("config.json")
    } else {
        args.config
    };
    let config = std::fs::read_to_string(config_path)?;
    let config: PrincipalConfig = serde_json::from_str(&config)?;

    let mut state = AppState::default();
    let (process_exit_tx, process_exit_rx) = unbounded_channel();

    let mut process_exits_actor = ProcessExitsActor::new();
    process_exits_actor.start(state.clone(), process_exit_tx.clone(), process_exit_rx);

    let mut hb_actor = HeartbeatActor::new(
        format!("{}/api/v1/principals", config.maestro_server_address),
        Duration::from_millis(config.heartbeat_interval_ms),
    );
    hb_actor.start(state.clone());
    let ct = CancellationToken::new();
    let ct_child = ct.child_token();

    // Replace with your target host
    let sse_url = format!(
        "{}/api/v1/notes?watch=true&field_selector=host=me",
        config.maestro_server_address
    );
    tokio::spawn(async move {
        println!("Subscribing to SSE from: {}", sse_url);

        let mut es = EventSource::get(sse_url);

        // We'll wrap our SSE loop in a `tokio::select!` to allow early exit on Ctrl-C.
        tokio::select! {
            // SSE subscription loop
            _ = async {
                while let Some(item) = es.next().await {
                    match item {
                        Ok(reqwest_eventsource::Event::Open) => info!("Connection Opened!"),
                        Ok(reqwest_eventsource::Event::Message(message)) => match message.event.as_str() {
                            "list" => {
                                debug!(notes = message.data, "Got list event");
                                // data is a list of notes
                                let notes: Vec<Note> = serde_json::from_str(&message.data)
                                    .expect("Should have received valid list of notes");
                                synchronize_state(&mut state, notes, process_exit_tx.clone()).await;
                            }
                            "added" => {
                                debug!("Got added event");
                                let new_note = serde_json::from_str(&message.data)
                                    .expect("Should receive valid note from added event");
                                let mut current_notes = state.notes.lock().await;
                                start_note(&mut current_notes, new_note, process_exit_tx.clone()).await;
                            }
                            "modified" => {
                                debug!("Got modified event");
                                let new_note: Note = serde_json::from_str(&message.data)
                                    .expect("Should receive valid note from modified event");
                                let mut current_notes = state.notes.lock().await;
                                if current_notes.get(&new_note.name).is_some() {
                                    // if the desired state is stop, stop process
                                    // if running and set state to terminated
                                    if matches!(new_note.state, NoteState::Terminating)
                                    {
                                        stop_note(&mut current_notes, &new_note.name).await;
                                    }
                                } else {
                                    warn!("Got modified event for untracked note");
                                }
                            }
                            "deleted" => {
                                debug!("Got deleted event");
                                let new_note: Note = serde_json::from_str(&message.data)
                                    .expect("Deleted event should give valid note");
                                let mut current_notes = state.notes.lock().await;
                                if current_notes.get(&new_note.name).is_some() {
                                    stop_note(&mut current_notes, &new_note.name).await;
                                }
                                current_notes.remove(&new_note.name);
                            }
                            _ => {
                                warn!("Unexpected message: {:#?}", message);
                            }
                        },
                        Err(err) => {
                            es.close();
                            panic!("Event source error: {:#?}", err)
                        }
                    }
                }
            } => {},
            _ = ct_child.cancelled() => {
                info!("Exiting SSE Task");
            }
        }
    });

    _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C signal, initiating shutdown.");
    ct.cancel();

    // Either the SSE stream ended or we caught Ctrl-C.
    // Proceed with your shutdown logic.

    println!("Disconnecting from SSE endpoint.");
    println!("Stopping heartbeats");
    hb_actor.stop().await;

    // Stop the process-exits actor last, or after you kill any processes
    // you *haven't* already killed.
    println!("Stopping process exits actor");
    process_exits_actor.stop().await;

    println!("Clean shutdown complete.");
    Ok(())
}

struct HeartbeatTaskState {
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

pub struct HeartbeatActor {
    url: String,
    heartbat_state: Option<HeartbeatTaskState>,
    heartbeat_interval: Duration,
}

impl HeartbeatActor {
    pub fn new(url: String, heartbeat_interval: Duration) -> Self {
        Self {
            url,
            heartbat_state: None,
            heartbeat_interval,
        }
    }

    pub fn start(&mut self, state: AppState) {
        let mut interval = time::interval(self.heartbeat_interval);
        let cancel = CancellationToken::new();
        let child = cancel.child_token();
        let url = self.url.clone();

        let jh = tokio::spawn(async move {
            let client = Client::new();
            {
                let notes = state.notes.lock().await;
                let _ =
                    send_heartbeat_forever_with_tokio_retry(&url, &client, &notes, child.clone())
                        .await;
            }
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let notes = state.notes.lock().await;
                        let _ = send_heartbeat_forever_with_tokio_retry(&url, &client, &notes, child.clone()).await;
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

async fn send_heartbeat_forever_with_tokio_retry(
    url: &str,
    client: &Client,
    notes: &HashMap<String, ManagedNote>,
    cancellation_token: CancellationToken,
) -> Result<(), anyhow::Error> {
    // Create an exponential backoff strategy with *no* max attempt limit
    // By default, if you don't call `.take(N)`, it will yield an infinite sequence of delays.
    let retry_strategy =
        ExponentialBackoff::from_millis(500).max_delay(std::time::Duration::from_secs(30)); // could be any max delay

    // Retry::spawn() will keep trying forever until it succeeds
    // or until the future is canceled externally.
    tokio::select! {
    _ = Retry::spawn(retry_strategy, || async {
            // If `heartbeat_task` fails, tokio-retry will handle the backoff and retry
            heartbeat_task(url, client, notes).await
        }) => {
            Ok(())
        }
    _ = cancellation_token.cancelled() => {Ok(())}
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
    let response = client.post(url).json(&heartbeat).send().await?;
    debug!("Heartbeat response: {:?}", response);
    Ok(())
}
