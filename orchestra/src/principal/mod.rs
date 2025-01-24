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
use reqwest_eventsource::{Event, EventSource};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tokio_stream::StreamExt;

use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
    time::{self, Interval},
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

/// Main entrypoint for the "principal".
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

    let config_data = std::fs::read_to_string(config_path)?;
    let config: PrincipalConfig = serde_json::from_str(&config_data)?;

    // Shared application state.
    let state = AppState::default();

    // Process exit channel + actor
    let (process_exit_tx, process_exit_rx) = unbounded_channel();
    let mut process_exits_actor = ProcessExitsActor::new();
    process_exits_actor.start(state.clone(), process_exit_tx.clone(), process_exit_rx);

    // SSE actor for note watch
    let sse_url = format!(
        "{}/api/v1/notes?watch=true&field_selector=host=me",
        config.maestro_server_address
    );
    let mut sse_actor = SSEActor::new(sse_url, process_exit_tx.clone());

    // Heartbeat actor
    let hb_url = format!("{}/api/v1/principals", config.maestro_server_address);
    let mut hb_actor =
        HeartbeatActor::new(hb_url, Duration::from_millis(config.heartbeat_interval_ms));

    // Start both actors
    sse_actor.start(state.clone());
    hb_actor.start(state.clone());

    // Wait for Ctrl-C
    info!("All actors started. Press Ctrl-C to shut down.");
    tokio::signal::ctrl_c().await?;
    info!("Ctrl-C received, initiating shutdown...");

    // Stop the SSE actor
    sse_actor.stop().await;
    info!("SSE actor stopped.");

    // Stop the Heartbeat actor
    hb_actor.stop().await;
    info!("Heartbeat actor stopped.");

    // Stop the process-exits actor last
    process_exits_actor.stop().await;
    info!("Process exits actor stopped.");

    info!("Clean shutdown complete.");
    Ok(())
}

/* --------------------------------------------------------------------------
SSEActor
-------------------------------------------------------------------------- */

/// An actor responsible for listening to the SSE stream of notes and
/// applying changes to the local state (start/stop processes, etc.).
pub struct SSEActor {
    /// SSE endpoint URL
    url: String,
    /// Handle to the spawned task
    handle: Option<JoinHandle<()>>,
    /// Cancellation token to shut down the SSE
    cancel_token: CancellationToken,
    /// MPSC channel for notifying about process exits
    process_exit_tx: UnboundedSender<(String, i32)>,
}

impl SSEActor {
    pub fn new(url: String, process_exit_tx: UnboundedSender<(String, i32)>) -> Self {
        Self {
            url,
            handle: None,
            cancel_token: CancellationToken::new(),
            process_exit_tx,
        }
    }

    /// Start the SSE actor. Spawns a task that keeps connecting to the SSE endpoint
    /// with exponential backoff, until cancellation is requested.
    pub fn start(&mut self, state: AppState) {
        let url = self.url.clone();
        let process_exit_tx = self.process_exit_tx.clone();
        let cancel_child = self.cancel_token.child_token();

        let handle = tokio::spawn(async move {
            info!("SSEActor started. Connecting to SSE at: {}", url);

            let retry_strategy =
                ExponentialBackoff::from_millis(500).max_delay(Duration::from_secs(30));

            // Outer loop for indefinite retry until canceled
            loop {
                let result = Retry::spawn(retry_strategy.clone(), || {
                    let child = cancel_child.child_token();
                    connect_and_stream_sse(&url, state.clone(), process_exit_tx.clone(), child)
                })
                .await;

                tokio::select! {
                    _ = cancel_child.cancelled() => {
                        info!("SSEActor canceled, exiting retry loop.");
                        break;
                    }
                    else => {
                        match result {
                            Ok(()) => {
                                info!("SSE connection ended gracefully, not retrying.");
                                break;
                            }
                            Err(e) => {
                                warn!("SSE connection failed after retries: {}. Will attempt again.", e);
                                // The loop will continue, re-attempting connection unless canceled
                            }
                        }
                    }
                }
            }

            info!("SSEActor main loop done.");
        });

        self.handle = Some(handle);
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            info!("Stopping SSEActor...");
            self.cancel_token.cancel();
            // Wait for the task to finish.
            let _ = handle.await;
        }
    }
}

/// Connects to the SSE endpoint, processes events until an error occurs or cancellation is requested.
/// If an error is returned, it allows the caller (the retry logic) to handle backoff and retry.
async fn connect_and_stream_sse(
    sse_url: &str,
    state: AppState,
    process_exit_tx: UnboundedSender<(String, i32)>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let mut es = EventSource::get(sse_url);
    info!("Connected to SSE endpoint: {}", sse_url);

    loop {
        tokio::select! {
            maybe_event = es.next() => {
                match maybe_event {
                    Some(Ok(Event::Open)) => {
                        debug!("SSE connection opened.");
                    }
                    Some(Ok(Event::Message(message))) => {
                        match message.event.as_str() {
                            "list" => {
                                debug!(notes = message.data, "Got list event");
                                let notes: Vec<Note> = serde_json::from_str(&message.data)
                                    .map_err(|e| anyhow!("Failed to parse 'list': {}", e))?;
                                synchronize_state(&mut state.clone(), notes, process_exit_tx.clone()).await;
                            }
                            "added" => {
                                debug!("Got added event");
                                let new_note: Note = serde_json::from_str(&message.data)
                                    .map_err(|e| anyhow!("Failed to parse 'added': {}", e))?;
                                let mut current_notes = state.notes.lock().await;
                                start_note(&mut current_notes, new_note, process_exit_tx.clone()).await;
                            }
                            "modified" => {
                                debug!("Got modified event");
                                let new_note: Note = serde_json::from_str(&message.data)
                                    .map_err(|e| anyhow!("Failed to parse 'modified': {}", e))?;
                                let mut current_notes = state.notes.lock().await;
                                if current_notes.get(&new_note.name).is_some() {
                                    if matches!(new_note.state, NoteState::Terminating) {
                                        stop_note(&mut current_notes, &new_note.name).await;
                                    }
                                } else {
                                    warn!("Got 'modified' event for untracked note: {}", new_note.name);
                                }
                            }
                            "deleted" => {
                                debug!("Got deleted event");
                                let deleted_note: Note = serde_json::from_str(&message.data)
                                    .map_err(|e| anyhow!("Failed to parse 'deleted': {}", e))?;
                                let mut current_notes = state.notes.lock().await;
                                if current_notes.get(&deleted_note.name).is_some() {
                                    stop_note(&mut current_notes, &deleted_note.name).await;
                                }
                                current_notes.remove(&deleted_note.name);
                            }
                            _ => {
                                warn!("Unexpected SSE message event: {:#?}", message);
                            }
                        }
                    }
                    Some(Err(err)) => {
                        warn!("SSE stream error: {}. Closing and returning error...", err);
                        es.close();
                        return Err(anyhow!("SSE stream error: {}", err));
                    }
                    None => {
                        warn!("SSE stream ended by server. Closing and returning error...");
                        es.close();
                        return Err(anyhow!("SSE stream ended by remote server"));
                    }
                }
            }
            _ = cancellation_token.cancelled() => {
                info!("SSEActor received cancellation; closing SSE stream.");
                es.close();
                return Ok(());
            }
        }
    }
}

/* --------------------------------------------------------------------------
HeartbeatActor
-------------------------------------------------------------------------- */

pub struct HeartbeatActor {
    url: String,
    heartbeat_interval: Duration,
    task_state: Option<HeartbeatTaskState>,
}

struct HeartbeatTaskState {
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl HeartbeatActor {
    pub fn new(url: String, heartbeat_interval: Duration) -> Self {
        Self {
            url,
            heartbeat_interval,
            task_state: None,
        }
    }

    pub fn start(&mut self, state: AppState) {
        let cancel_token = CancellationToken::new();
        let child_token = cancel_token.child_token();
        let url = self.url.clone();
        let interval = time::interval(self.heartbeat_interval);

        let task = tokio::spawn(async move {
            run_heartbeat_loop(&url, interval, state, child_token).await;
        });

        self.task_state = Some(HeartbeatTaskState {
            task,
            cancellation_token: cancel_token,
        });
    }

    pub async fn stop(&mut self) {
        if let Some(task_state) = self.task_state.take() {
            info!("Stopping HeartbeatActor...");
            task_state.cancellation_token.cancel();
            let _ = task_state.task.await;
        }
    }
}

/// The main heartbeat loop: runs until cancelled.
async fn run_heartbeat_loop(
    url: &str,
    mut interval: Interval,
    state: AppState,
    cancellation_token: CancellationToken,
) {
    let client = Client::new();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let notes = state.notes.lock().await;
                let _ = send_heartbeat_with_retries(url, &client, &notes, cancellation_token.child_token()).await;
            }
            _ = cancellation_token.cancelled() => {
                info!("HeartbeatActor cancelled.");
                break;
            }
        }
    }
    info!("HeartbeatActor loop ended.");
}

/// Sends one heartbeat, retrying until it succeeds or we get cancelled.
async fn send_heartbeat_with_retries(
    url: &str,
    client: &Client,
    notes: &HashMap<String, ManagedNote>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let retry_strategy = ExponentialBackoff::from_millis(500).max_delay(Duration::from_secs(30));

    tokio::select! {
        result = Retry::spawn(retry_strategy, || async {
            send_heartbeat_once(url, client, notes).await
        }) => {
            result.map_err(|e| anyhow!("Heartbeat retries failed: {e}"))
        }
        _ = cancellation_token.cancelled() => {
            debug!("Heartbeat send cancelled.");
            Ok(())
        }
    }
}

/// Sends a single heartbeat without internal retry logic.
async fn send_heartbeat_once(
    url: &str,
    client: &Client,
    notes: &HashMap<String, ManagedNote>,
) -> Result<()> {
    let notes_dto = notes
        .iter()
        .map(|(_name, managed)| HeartbeatNoteDto {
            symphony: managed.note.symphony.clone(),
            name: managed.note.name.clone(),
            state: managed.note.state.clone(),
        })
        .collect();

    let heartbeat = HeartbeatDto {
        name: "me".to_string(),
        notes: notes_dto,
    };

    let response = client.post(url).json(&heartbeat).send().await?;
    debug!("Heartbeat response: {:?}", response.status());
    Ok(())
}
