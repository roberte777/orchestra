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

    let state = AppState::default();
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

    start_sse_subscriber(sse_url, state, process_exit_tx, ct_child);

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

/// Spawns a task to continuously connect to the SSE endpoint, read events, and retry on failure.
/// Cancels cleanly if the provided `cancellation_token` is triggered.
pub fn start_sse_subscriber(
    sse_url: String,
    state: AppState,
    process_exit_tx: UnboundedSender<(String, i32)>,
    cancellation_token: CancellationToken,
) {
    // Spawn the SSE subscription on its own task
    tokio::spawn(async move {
        info!("Starting SSE subscriber for URL: {}", sse_url);

        // An infinite loop that will keep trying to connect to the SSE endpoint
        // until cancellation is requested.
        let retry_strategy =
            ExponentialBackoff::from_millis(500).max_delay(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("SSE subscriber received cancellation signal, exiting.");
                    break;
                }
                result = Retry::spawn(retry_strategy.clone(), || {
                    // Our SSE connection attempt
                    async {
                        let child_token = cancellation_token.child_token();
                        connect_and_stream_sse(
                            &sse_url,
                            state.clone(),
                            process_exit_tx.clone(),
                            child_token,
                        )
                        .await
                    }
                }) => {
                    match result {
                        Ok(()) => {
                            // If connect_and_stream_sse returned Ok, that means
                            // it exited gracefully (likely a normal shutdown).
                            info!("SSE connection ended gracefully. Will not reconnect.");
                            break;
                        }
                        Err(e) => {
                            // If connect_and_stream_sse returned an error, we let
                            // tokio-retry handle the exponential backoff and then retry.
                            warn!("SSE connection failed: {}. Will retry...", e);
                            // The loop continues, which triggers Retry::spawn again
                        }
                    }
                }
            }
        }

        info!("SSE subscriber task has fully exited.");
    });
}

/// Connects to the SSE endpoint, processes events until an error occurs or cancellation is requested.
/// If an error is returned, it allows the caller (the retry loop) to back off and retry.
async fn connect_and_stream_sse(
    sse_url: &str,
    state: AppState,
    process_exit_tx: UnboundedSender<(String, i32)>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    // Create the EventSource.
    // If the server is unreachable or returns an error upon initial connection,
    // EventSource::get(...) will succeed, but reading from `es.next()` might fail immediately.
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
                        // We got an error from the SSE stream. This will break
                        // and return an error to the retry logic.
                        warn!("SSE stream error: {}. Closing and retrying...", err);
                        es.close();
                        return Err(anyhow!("SSE stream error: {}", err));
                    }
                    None => {
                        // The stream ended normally (the server closed the connection).
                        // Return an error so that we can attempt reconnect, or interpret
                        // as a graceful end if you prefer. Returning an error will cause
                        // the retry strategy to engage.
                        warn!("SSE stream ended. Will attempt to reconnect.");
                        es.close();
                        return Err(anyhow!("SSE stream ended by remote server"));
                    }
                }
            }
            _ = cancellation_token.cancelled() => {
                info!("Cancellation token triggered for SSE connection. Closing stream.");
                es.close();
                return Ok(());
            }
        }
    }
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
