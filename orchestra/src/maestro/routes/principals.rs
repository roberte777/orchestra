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
use tracing::info;
use watcher::EventType;

use crate::maestro::{
    models::{NoteState, Principal, PrincipalState, SymphonyState},
    AppState,
};

async fn principal_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(heartbeat): Json<HeartbeatDto>,
) {
    info!("got heartbeat: {:?}", heartbeat);
    let principal_repo = app_state.principal_repository.lock().await;
    let note_repo = app_state.note_repository.lock().await;
    let symphony_repo = app_state.symphony_repository.lock().await;

    for note in heartbeat.notes {
        // remove terminated notes from state
        if matches!(note.state, NoteState::Terminated) {
            // collect data
            let note = match note_repo.get_note(&note.name).await {
                Some(note) => note,
                None => continue,
            };
            let symphony_name = note.symphony.clone();
            let mut symphony = symphony_repo.get_symphony(&symphony_name).await.unwrap();

            // update symphony
            symphony.notes.retain(|n| *n != note.name);
            let should_remove = matches!(symphony.state(), SymphonyState::Terminating)
                && symphony.notes().is_empty();
            _ = symphony_repo.update_symphony(symphony).await;
            //update note
            note_repo.remove_note(&note.name).await;
            let event = watcher::Event {
                event_type: EventType::Deleted,
                resource: note,
            };
            app_state.watch_manager.lock().await.notify_note(event);

            // remove symphony if stop requested and all notes gone
            if should_remove {
                symphony_repo.remove_symphony(&symphony_name).await;
            }
        } else {
            let mut note_update = note_repo
                .get_note(&note.name)
                .await
                .expect("Should get valid note from principal heartbeat");
            // terminating notes can only be updated to terminated
            if matches!(note_update.state, NoteState::Terminating) {
                continue;
            }
            note_update.state = note.state;
            note_repo.update_note(note_update).await;
        }
    }

    let principal = Principal {
        host: heartbeat.name,
        capabilities: Vec::new(),
        state: PrincipalState::Ready,
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Should be able to get a valid duration since unix epoch")
            .as_secs(),
    };

    principal_repo.upsert_principal(principal).await;
}

async fn get_principals(State(app_state): State<Arc<AppState>>) -> Json<Vec<Principal>> {
    let principal_repo = app_state.principal_repository.lock().await;
    let principals = principal_repo.get_all_principals().await;
    Json(principals)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(principal_heartbeat))
        .route("/", get(get_principals))
}

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
