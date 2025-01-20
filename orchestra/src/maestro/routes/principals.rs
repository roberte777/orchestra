use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use watcher::EventType;

use crate::maestro::{
    models::{DesiredState, NoteState, Principal, PrincipalState},
    AppState,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartbeatDto {
    pub name: String,
    pub notes: Vec<HeartbeatNoteDto>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartbeatNoteDto {
    pub name: String,
    pub state: NoteState,
}

async fn principal_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(heartbeat): Json<HeartbeatDto>,
) {
    info!("got heartbeat: {:?}", heartbeat);

    let principal_repo = app_state.principal_repository.lock().await;
    let note_repo = app_state.note_repository.lock().await;
    let symphony_repo = app_state.symphony_repository.lock().await;

    // 1) Upsert the principal
    let principal = Principal {
        host: heartbeat.name.clone(),
        capabilities: vec![],
        state: PrincipalState::Ready,
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration since epoch")
            .as_secs(),
    };
    principal_repo.upsert_principal(principal).await;
    drop(principal_repo);

    // 2) For each note in the heartbeat, update or remove
    for note_heartbeat in heartbeat.notes {
        // If note is "Terminated," we might remove it entirely from the store
        if matches!(note_heartbeat.state, NoteState::Terminated) {
            if let Some(note) = note_repo.get_note(&note_heartbeat.name).await {
                // Remove the note from the store
                note_repo.remove_note(&note.name).await;
                // Fire "Deleted" watch event
                app_state
                    .watch_manager
                    .lock()
                    .await
                    .notify_note(watcher::Event {
                        event_type: watcher::EventType::Deleted,
                        resource: note.clone(),
                    });

                // Also remove from the symphony’s note list if needed:
                if let Some(mut sym) = symphony_repo.get_symphony(&note.symphony).await {
                    sym.notes.retain(|n| n != &note_heartbeat.name);
                    // If that symphony has no notes and is desired_state=Stop, you might remove it entirely:
                    if sym.notes.is_empty() && matches!(sym.desired_state(), DesiredState::Stop) {
                        symphony_repo.remove_symphony(&sym.name).await;
                        // If you want watchers for the symphony:
                        // app_state.watch_manager.lock().await.notify_symphony(...);
                    } else {
                        // Otherwise, just update the symphony’s note list
                        symphony_repo.update_symphony(sym).await;
                    }
                }
            }
        } else {
            // The note is still alive, so update its state
            if let Some(mut existing_note) = note_repo.get_note(&note_heartbeat.name).await {
                existing_note.state = note_heartbeat.state;
                note_repo.update_note(existing_note.clone()).await;
                // You can optionally fire watchers for "modified" here, if you prefer.
                // For example:
                // app_state.watch_manager.lock().await.notify_note(watcher::Event {
                //     event_type: watcher::EventType::Modified,
                //     resource: existing_note,
                // });
            }
        }
    }

    drop(note_repo);
    drop(symphony_repo);
}

async fn get_principals(State(app_state): State<Arc<AppState>>) -> Json<Vec<Principal>> {
    let principal_repo = app_state.principal_repository.lock().await;
    let all = principal_repo.get_all_principals().await;
    Json(all)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(principal_heartbeat))
        .route("/", get(get_principals))
}
