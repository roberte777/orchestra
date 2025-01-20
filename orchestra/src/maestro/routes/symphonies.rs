use std::{collections::HashMap, sync::Arc};

use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use watcher::{Event, EventType};

use crate::maestro::{
    models::{DesiredState, Note, NoteState, RestartPolicy, Symphony},
    AppState,
};

#[derive(Clone, Deserialize)]
pub struct SymphonyDto {
    pub name: String,
    pub notes: Vec<NoteDto>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct NoteDto {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
}

impl NoteDto {
    pub fn to_data_obj(&self, symphony_name: &str) -> Note {
        Note {
            name: self.name.clone(),
            description: self.description.clone(),
            host: self.host.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            env: self.env.clone(),
            restart_policy: self.restart_policy.clone(),
            state: NoteState::Pending,
            desired_state: DesiredState::Run,
            symphony: symphony_name.to_string(),
        }
    }
}

impl SymphonyDto {
    pub fn to_symphony(&self) -> Symphony {
        let note_names = self.notes.iter().map(|n| n.name.clone()).collect();
        Symphony {
            name: self.name.clone(),
            notes: note_names,
            desired_state: DesiredState::Run,
        }
    }
}

#[derive(Serialize)]
struct SymphonyReturn {
    name: String,
    notes: Vec<Note>,
}

/// Get all symphonies plus their notes
#[debug_handler]
async fn get_all_symphonies(State(state): State<Arc<AppState>>) -> Json<Vec<SymphonyReturn>> {
    let s_repo = state.symphony_repository.lock().await;
    let n_repo = state.note_repository.lock().await;
    let symphonies = s_repo.get_all_symphonies().await;
    let mut final_symphonies = Vec::new();

    for sym in symphonies {
        let mut notes_for_sym = Vec::new();
        for note_name in &sym.notes {
            if let Some(n) = n_repo.get_note(note_name).await {
                notes_for_sym.push(n);
            }
        }
        final_symphonies.push(SymphonyReturn {
            name: sym.name,
            notes: notes_for_sym,
        });
    }
    Json(final_symphonies)
}

/// Create and “start” (desired_state=Run) a new Symphony **and** its notes in one shot.
pub async fn create_symphony(
    State(app_state): State<Arc<AppState>>,
    Json(dto): Json<SymphonyDto>,
) -> impl IntoResponse {
    let repo = app_state.symphony_repository.lock().await;

    let new_symphony = dto.to_symphony();
    let new_notes: Vec<Note> = dto
        .notes
        .iter()
        .map(|n| n.to_data_obj(&new_symphony.name))
        .collect();

    // Perform the single transaction
    let result = repo
        .add_symphony_with_notes(new_symphony.clone(), new_notes.clone())
        .await;
    drop(repo); // We release the lock before watchers

    if let Err(err_msg) = result {
        return (StatusCode::BAD_REQUEST, err_msg).into_response();
    }

    // Notify watchers for each newly added note
    let mut wm = app_state.watch_manager.lock().await;
    for note in new_notes {
        wm.notify_note(Event {
            event_type: EventType::Added,
            resource: note,
        });
    }
    // If you also want to watch symphonies, you can do something like:
    // wm.notify_symphony(Event {
    //     event_type: EventType::Added,
    //     resource: new_symphony,
    // });

    StatusCode::OK.into_response()
}

/// Stop a Symphony (set desired_state=Stop) and also set desired_state=Stop on all its notes.
pub async fn stop_symphony(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let sym_repo = app_state.symphony_repository.lock().await;
    let note_repo = app_state.note_repository.lock().await;

    // 1) Look up the symphony
    let symphony = match sym_repo.get_symphony(&name).await {
        Some(s) => s,
        None => return StatusCode::NOT_FOUND,
    };

    // 2) For each note, set desired_state=Stop
    for note_name in symphony.notes() {
        // If success, push a "Modified" event
        if note_repo.stop_note(note_name).await {
            if let Some(stopped_note) = note_repo.get_note(note_name).await {
                let event = Event {
                    event_type: EventType::Modified,
                    resource: stopped_note,
                };
                app_state.watch_manager.lock().await.notify_note(event);
            }
        }
    }

    // 3) Now set desired_state=Stop on the symphony
    sym_repo.stop_symphony(&name).await;
    drop(sym_repo);
    drop(note_repo);

    // If you have watchers for the Symphony
    // let event = Event {
    //     event_type: EventType::Modified,
    //     resource: symphony,
    // };
    // app_state.watch_manager.lock().await.notify_symphony(event);

    StatusCode::OK
}

/// Optional: remove a symphony and all its notes in one shot
pub async fn remove_symphony(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let repo = app_state.symphony_repository.lock().await;

    // This calls the new “remove_symphony_and_notes” method in the store
    let success = repo.remove_symphony_and_notes(&name).await;
    drop(repo);

    if !success {
        return StatusCode::NOT_FOUND;
    }

    // If you want watchers for removed notes or symphony, you would need
    // to fetch them prior to removing them (to know which ones to "Deleted"-notify).
    // For brevity, not shown here.

    StatusCode::OK
}

/// Return all notes for a given symphony, if desired
pub async fn get_notes_for_symphony(
    Path(symphony_name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let s_repo = app_state.symphony_repository.lock().await;
    let n_repo = app_state.note_repository.lock().await;
    let sym = s_repo.get_symphony(&symphony_name).await;
    drop(s_repo);

    if let Some(symphony) = sym {
        let mut notes_for_symph = Vec::new();
        for n in symphony.notes() {
            if let Some(note) = n_repo.get_note(n).await {
                notes_for_symph.push(note);
            }
        }
        Json(notes_for_symph).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // List all symphonies
        .route("/", get(get_all_symphonies).post(create_symphony))
        // Stop a symphony
        .route("/{name}/stop", post(stop_symphony))
        // Remove entire symphony (optional route)
        .route("/{name}/remove", post(remove_symphony))
        // Get all notes for that symphony
        .route("/{symphony_name}/notes", get(get_notes_for_symphony))
}
