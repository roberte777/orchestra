use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    debug_handler,
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::maestro::{
    models::{NoteState, Principal, PrincipalState},
    AppState,
};

async fn principal_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(heartbeat): Json<HeartbeatDto>,
) {
    info!("got heartbeat: {:?}", heartbeat);
    let principal_repo = app_state.principal_repository.lock().await;
    let note_repo = app_state.note_repository.lock().await;

    for note in heartbeat.notes {
        let mut note_update = note_repo
            .get_note(&note.name)
            .await
            .expect("Should get valid note from principal heartbeat");

        note_update.state = note.state;
        note_repo.update_note(note_update).await;
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
