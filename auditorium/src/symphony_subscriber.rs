use std::collections::HashMap;

use futures::StreamExt;
use orchestra::maestro::{dto::SymphonyWithNotes, models::NoteState};
use reqwest_eventsource::{Event, EventSource};
use tokio::{
    sync::{broadcast::Sender, oneshot},
    task::JoinHandle,
};
use tracing::{debug, info, warn};

use crate::{
    tracked_symphonies::{AuditoriumResourceState, TrackedSymphony},
    AppState,
};

/// A struct that encapsulates the logic for subscribing to SSE and managing
/// the lifecycle of that subscription.
pub struct SymphonySubscriber {
    /// The URL used to subscribe to SSE.
    sse_url: String,
    /// Shared application state.
    state: AppState,
    /// Channel to signal process exit (or do other control).
    ping: Sender<()>,
    /// A handle to the background SSE task, so we can stop it.
    task_handle: Option<JoinHandle<()>>,
    /// A oneshot channel used to request shutdown.
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl SymphonySubscriber {
    /// Create a new SseSubscriber.
    pub fn new(host: String, state: AppState, ping: Sender<()>) -> Self {
        // Construct the SSE URL in here or accept it as a parameter
        let sse_url = format!("{}/api/v1/symphonies?watch=true", host);

        SymphonySubscriber {
            sse_url,
            state,
            ping,
            task_handle: None,
            shutdown_tx: None,
        }
    }

    /// Start the SSE subscription task.
    pub fn start(&mut self) {
        // Create a one-shot channel we’ll use for shutdown signals.
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let sse_url_clone = self.sse_url.clone();
        let state = self.state.clone();
        let ping = self.ping.clone();

        // Spawn the background task.
        let handle = tokio::spawn(async move {
            info!("Subscribing to SSE from: {}", sse_url_clone);

            let mut es = EventSource::get(sse_url_clone);

            // SSE subscription loop.
            // We'll wrap our logic in a select so we can break out when `shutdown_rx` fires.
            tokio::select! {
                _ = async {
                    while let Some(item) = es.next().await {
                        match item {
                            Ok(Event::Open) => {
                                info!("Connection Opened!");
                            }
                            Ok(Event::Message(message)) => {
                                match message.event.as_str() {
                                    "list" => {
                                        debug!("Got list event: {}", message.data);
                                        let symphonies: Vec<SymphonyWithNotes> = serde_json::from_str(&message.data)
                                            .expect("Should have received valid list of symphonies");
                                        merge_symphonies(&state, symphonies).await;
                                        let _ = ping.send(());
                                    }
                                    "added" => {
                                        debug!("Got added event");
                                        let symphony: SymphonyWithNotes = serde_json::from_str(&message.data)
                                            .expect("Should have received valid symphony");
                                        merge_symphonies(&state, vec![symphony]).await;
                                        let _  = ping.send(());

                                    }
                                    "modified" => {
                                        debug!("Got modified event");
                                        let symphony: SymphonyWithNotes = serde_json::from_str(&message.data)
                                            .expect("Should have received valid symphony");
                                        merge_symphonies(&state, vec![symphony]).await;
                                        let _  = ping.send(());
                                    }
                                    "deleted" => {
                                        debug!("Got deleted event");
                                        let symphony: SymphonyWithNotes = serde_json::from_str(&message.data)
                                            .expect("Should have received valid symphony");
                                        let mut tracked = state.tracked_symphonies.lock().await;
                                        if let Some(symphony) = tracked.symphonies.get_mut(&symphony.name) {
                                            symphony.auditorium_state = AuditoriumResourceState::Stopped;
                                            for note in &mut symphony.notes {
                                                note.auditorium_state = AuditoriumResourceState::Stopped;
                                            }
                                        }
                                        let _  = ping.send(());
                                    }
                                    _ => {
                                        warn!("Unexpected message event: {:#?}", message);
                                    }
                                }
                            }
                            Err(err) => {
                                es.close();
                                panic!("Event source error: {:#?}", err)
                            }
                        }
                    }
                } => {},
                _ = &mut shutdown_rx => {
                    // If we get a shutdown signal, exit the task.
                    info!("Shutdown signal received for SSE task.");
                    es.close();
                }
            }

            info!("Exiting SSE subscription task.");
        });

        self.task_handle = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);
    }

    /// Stop the SSE subscription task.
    ///
    /// Sends a shutdown signal, then awaits the underlying task to ensure clean exit.
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()); // We can ignore result since the task might have already ended.
        }

        if let Some(handle) = self.task_handle.take() {
            // Wait for the task to finish.
            if let Err(err) = handle.await {
                warn!("Error stopping SSE task: {:?}", err);
            }
        }

        info!("SSE subscriber stopped.");
    }
}

async fn merge_symphonies(app_state: &AppState, maestro_symphonies: Vec<SymphonyWithNotes>) {
    // 1) Copy the user-tracked symphonies from the in-memory store
    let mut store = app_state.tracked_symphonies.lock().await;
    let mut tracked_list: Vec<TrackedSymphony> = store.symphonies.values().cloned().collect();

    // If we got a valid list from Maestro, we can attempt to match them
    // Build a map (by name) for quick lookup
    let mut maestro_map = HashMap::new();
    for ms in maestro_symphonies {
        maestro_map.insert(ms.name.clone(), ms);
    }

    // For each tracked symphony, see if it exists in maestro
    for ts in tracked_list.iter_mut() {
        if let Some(maestro_sym) = maestro_map.get(&ts.name) {
            // Mark that we found it
            ts.auditorium_state = AuditoriumResourceState::Running;

            // Now also merge note states
            // We'll build a map of maestro notes for quick lookup
            let mut maestro_notes_map = HashMap::new();
            for note in &maestro_sym.notes {
                maestro_notes_map.insert(note.name.clone(), note);
            }
            for tnote in ts.notes.iter_mut() {
                if let Some(mnote) = maestro_notes_map.get(&tnote.name) {
                    // It's present in Maestro
                    tnote.auditorium_state = AuditoriumResourceState::Running;
                    tnote.state = mnote.state.clone();
                } else {
                    // It's not in Maestro
                    tnote.auditorium_state = AuditoriumResourceState::Stopped;
                    tnote.state = NoteState::Terminated;
                }
            }
        } else {
            // The symphony is missing in Maestro
            ts.auditorium_state = AuditoriumResourceState::Stopped;
            // Possibly also mark all notes as missing
            for tnote in ts.notes.iter_mut() {
                tnote.auditorium_state = AuditoriumResourceState::Stopped;
                tnote.state = NoteState::Terminated;
            }
        }
    }

    // update store
    for symphony in tracked_list {
        *store
            .symphonies
            .get_mut(&symphony.name)
            .expect("Tracked symphony must exist") = symphony.clone();
    }
}
