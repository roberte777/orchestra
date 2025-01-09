use std::{collections::HashMap, sync::Arc};

use axum::{body::Body, extract::State, routing::get, Json, Router};
use serde::Deserialize;

use crate::maestro::{
    models::{DesiredState, Note, NoteState, Symphony},
    AppState,
};

// step 1: start a symphony and stop a symphony

/// Create and start a Symphony
pub async fn start_symphony(
    Json(symphony_dto): Json<SymphonyDto>,
    State(app_state): State<AppState>,
) {
    let symphony = symphony_dto.to_data_obj();
    let notes = symphony_dto
        .notes
        .iter()
        .map(|n| n.to_data_obj(&symphony.name))
        .collect::<Vec<Note>>();
    {
        let symphony_repo = app_state.symphony_repository.lock().await;
        let symphony_name = symphony.name();
        symphony_repo.add_symphony(symphony).await;
        symphony_repo.start_symphony(&symphony_name).await;
    }
    {
        let notes_repo = app_state.note_repository.lock().await;
        for note in notes.clone() {
            let note_name = note.name.clone();
            notes_repo.add_note(note).await;
            notes_repo.start_note(&note_name).await;
        }
    }
    let mut wm = app_state.watch_manager.lock().await;
    for note in notes {
        wm.notify_note(watcher::Event {
            event_type: watcher::EventType::Added,
            resource: note,
        });
    }
}

/// Starts an existing symphony
pub async fn start_symphony_by_id() {}

pub async fn get_notes_for_symphony() {}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/symphonies/:symphony_name/notes",
        get(get_notes_for_symphony),
    )
}

#[derive(Clone, Deserialize, Debug)]
pub struct NoteDto {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: String,
}

impl NoteDto {
    pub fn to_data_obj(&self, symphony_name: &str) -> Note {
        Note {
            name: self.name.clone(),
            description: self.description.clone(),
            host: self.host.clone(),
            env: self.env.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            restart_policy: self.restart_policy.clone(),
            state: NoteState::Pending,
            symphony: symphony_name.to_string(),
            desired_state: DesiredState::Run,
        }
    }
}

#[derive(Clone)]
pub struct SymphonyDto {
    pub name: String,
    pub notes: Vec<NoteDto>,
}

impl SymphonyDto {
    pub fn to_data_obj(&self) -> Symphony {
        let notes = self.notes.iter().map(|n| n.name.clone()).collect();
        Symphony {
            notes,
            name: self.name.clone(),
            desired_state: DesiredState::Run,
        }
    }
}
