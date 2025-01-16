use anyhow::Result;
use orchestra::{
    maestro::models::{DesiredState, Note, NoteState},
    principal::{
        note_process::{start_note, stop_note, synchronize_state},
        process_exits_actor::ProcessExitsActor,
        AppState, HeartbeatActor,
    },
};
use reqwest_eventsource::EventSource;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut state = AppState::default();
    let (process_exit_tx, process_exit_rx) = unbounded_channel();

    let mut process_exits_actor = ProcessExitsActor::new();
    process_exits_actor.start(state.clone(), process_exit_tx.clone(), process_exit_rx);

    let mut hb_actor = HeartbeatActor::new("http://localhost:3000/api/v1/principals".to_string());
    hb_actor.start(state.clone());

    // Replace with your target host
    let host = "http://localhost:3000";
    let sse_url = format!("{}/api/v1/notes?watch=true&field_selector=host=me", host);

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
                            synchronize_state(&mut state, notes).await;
                        }
                        "added" => {
                            debug!("Got list event");
                            let new_note = serde_json::from_str(&message.data)
                                .expect("Should receive valid note from added event");
                            let mut current_notes = state.notes.lock().await;
                            start_note(&mut current_notes, new_note, process_exit_tx.clone()).await;
                        }
                        "modified" => {
                            debug!("Got list event");
                            let new_note: Note = serde_json::from_str(&message.data)
                                .expect("Should receive valid note from modified event");
                            let mut current_notes = state.notes.lock().await;
                            if let Some(managed_note) = current_notes.get(&new_note.name) {
                                if matches!(new_note.desired_state, DesiredState::Run)
                                    && !matches!(managed_note.note.state, NoteState::Running)
                                {
                                    start_note(&mut current_notes, new_note, process_exit_tx.clone()).await;
                                } else if matches!(new_note.desired_state, DesiredState::Stop)
                                    && matches!(managed_note.note.state, NoteState::Running)
                                {
                                    stop_note(&mut current_notes, &new_note.name).await;
                                }
                            } else {
                                warn!("Got modified event for untracked note");
                            }
                        }
                        "deleted" => {
                            debug!("Got list event");
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

        // Exit on Ctrl-C
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl-C signal, initiating shutdown.");
        }
    }

    // Either the SSE stream ended or we caught Ctrl-C.
    // Proceed with your shutdown logic.

    println!("Disconnecting from SSE endpoint.");
    hb_actor.stop().await;

    // Stop the process-exits actor last, or after you kill any processes
    // you *haven't* already killed.
    process_exits_actor.stop().await;

    println!("Clean shutdown complete.");
    Ok(())
}
