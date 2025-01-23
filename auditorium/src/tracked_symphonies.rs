use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use orchestra::{
    maestro::models::{NoteState, RestartPolicy},
    CreateSymphony, SymphonyWithNotes,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// We can define new "meta-states" for how the Auditorium sees a symphony's or note's status.
/// E.g., if the symphony does not exist in Maestro, we mark it as "MissingInMaestro".
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum AuditoriumResourceState {
    Stopped,
    Running,
}

/// Our "tracked" version of a Note
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TrackedNote {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
    pub state: NoteState,
    // Possibly store the last known "state" from Maestro
    pub auditorium_state: AuditoriumResourceState,
}

/// Our "tracked" version of a Symphony
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TrackedSymphony {
    pub name: String,
    pub notes: Vec<TrackedNote>,
    /// We'll store the "Auditorium's perspective" on the symphony's overall state.
    pub auditorium_state: AuditoriumResourceState,
}

/// Simple in-memory store. In production you might load/save from a file, or use a DB.
#[derive(Default)]
pub struct InMemoryAuditoriumStore {
    pub symphonies: HashMap<String, TrackedSymphony>,
}

impl InMemoryAuditoriumStore {
    /// Provide some default data for demonstration.
    pub fn with_defaults() -> Self {
        let mut store = InMemoryAuditoriumStore::default();

        let sym1 = TrackedSymphony {
            name: "Symphony1".to_string(),
            notes: vec![
                TrackedNote {
                    name: "Note1".to_string(),
                    description: "First note".to_string(),
                    host: "me".to_string(),
                    command: "echo".to_string(),
                    args: vec!["Hello".into(), "World".into()],
                    env: {
                        let mut m = HashMap::new();
                        m.insert("VAR1".into(), "value1".into());
                        m
                    },
                    restart_policy: RestartPolicy::Never,
                    state: NoteState::Terminated,
                    auditorium_state: AuditoriumResourceState::Stopped,
                },
                TrackedNote {
                    name: "Note2".to_string(),
                    description: "First note".to_string(),
                    host: "me".to_string(),
                    command: "ping".to_string(),
                    args: vec!["127.0.0.1".to_string()],
                    env: {
                        let mut m = HashMap::new();
                        m.insert("VAR1".into(), "value1".into());
                        m
                    },
                    restart_policy: RestartPolicy::Never,
                    state: NoteState::Terminated,
                    auditorium_state: AuditoriumResourceState::Stopped,
                },
            ],
            auditorium_state: AuditoriumResourceState::Stopped,
        };
        let sym2 = TrackedSymphony {
            name: "MyHardcodedSymphony".to_string(),
            notes: vec![],
            auditorium_state: AuditoriumResourceState::Stopped,
        };

        store.symphonies.insert(sym1.name.clone(), sym1);
        store.symphonies.insert(sym2.name.clone(), sym2);
        store
    }
}

/// Merge the user’s tracked list with the “real state” from Maestro.
async fn fetch_and_merge_symphonies(app_state: &AppState) -> Vec<TrackedSymphony> {
    // 1) Copy the user-tracked symphonies from the in-memory store
    let store = app_state.tracked_symphonies.lock().await;
    let mut tracked_list: Vec<TrackedSymphony> = store.symphonies.values().cloned().collect();
    drop(store);

    // 2) Fetch the actual list from Maestro
    //    (GET /api/v1/symphonies from the maestro_url)
    let url = format!("{}/api/v1/symphonies", app_state.maestro_url);
    let result = app_state.client.get(url).send().await;

    let maestro_symphonies: Option<Vec<SymphonyWithNotes>> = match result {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<Vec<SymphonyWithNotes>>().await {
                    Ok(data) => Some(data),
                    Err(e) => {
                        eprintln!("Failed to decode maestro symphonies: {:?}", e);
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            eprintln!("Failed to request maestro symphonies: {:?}", e);
            None
        }
    };

    // If we got a valid list from Maestro, we can attempt to match them
    if let Some(maestro_list) = maestro_symphonies {
        // Build a map (by name) for quick lookup
        let mut maestro_map = HashMap::new();
        for ms in maestro_list {
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
    } else {
        // If we failed to fetch from Maestro, we won't override anything
        // (just treat them as missing from Maestro or unknown)
        for ts in tracked_list.iter_mut() {
            ts.auditorium_state = AuditoriumResourceState::Stopped;
            for tnote in ts.notes.iter_mut() {
                tnote.auditorium_state = AuditoriumResourceState::Stopped;
                tnote.state = NoteState::Terminated;
            }
        }
    }

    tracked_list
}

// Add your typical CRUD handlers for the tracked symphonies

/// GET /api/v1/tracked-symphonies
/// Return the user’s tracked list, merged with Maestro state
async fn get_tracked_symphonies(State(app_state): State<AppState>) -> Json<Vec<TrackedSymphony>> {
    let merged = fetch_and_merge_symphonies(&app_state).await;
    Json(merged)
}

/// POST /api/v1/tracked-symphonies
/// Add a new user-tracked symphony
async fn create_tracked_symphony(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateSymphony>,
) -> StatusCode {
    let mut store = app_state.tracked_symphonies.lock().await;
    if store.symphonies.contains_key(&payload.name) {
        // Already exists
        return StatusCode::CONFLICT;
    }
    let mut tracked_notes = Vec::new();
    for note in payload.notes {
        let tracked_note = TrackedNote {
            name: note.name,
            description: note.description,
            host: note.host,
            command: note.command,
            args: note.args,
            env: note.env,
            restart_policy: note.restart_policy,
            state: NoteState::Terminated,
            auditorium_state: AuditoriumResourceState::Stopped,
        };

        tracked_notes.push(tracked_note);
    }
    let tracked_symphony = TrackedSymphony {
        name: payload.name,
        notes: tracked_notes,
        auditorium_state: AuditoriumResourceState::Stopped,
    };
    store
        .symphonies
        .insert(tracked_symphony.name.clone(), tracked_symphony);
    app_state.symphony_tx.send(()).unwrap();
    StatusCode::CREATED
}

/// PUT /api/v1/tracked-symphonies/:name
/// Update an existing user-tracked symphony
async fn update_tracked_symphony(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<CreateSymphony>,
) -> StatusCode {
    println!("{}", name);
    println!("{:?}", payload);
    let mut store = app_state.tracked_symphonies.lock().await;
    if !store.symphonies.contains_key(&name) {
        return StatusCode::NOT_FOUND;
    }
    let mut tracked_notes = Vec::new();
    for note in payload.notes {
        let tracked_note = TrackedNote {
            name: note.name,
            description: note.description,
            host: note.host,
            command: note.command,
            args: note.args,
            env: note.env,
            restart_policy: note.restart_policy,
            state: NoteState::Terminated,
            auditorium_state: AuditoriumResourceState::Stopped,
        };

        tracked_notes.push(tracked_note);
    }
    let tracked_symphony = TrackedSymphony {
        name: payload.name,
        notes: tracked_notes,
        auditorium_state: AuditoriumResourceState::Stopped,
    };
    store
        .symphonies
        .insert(tracked_symphony.name.clone(), tracked_symphony.clone());
    // if the name is changing, insert and remove
    if tracked_symphony.name != name {
        store.symphonies.remove(&name);
    }
    app_state.symphony_tx.send(()).unwrap();
    StatusCode::OK
}

/// DELETE /api/v1/tracked-symphonies/:name
async fn delete_tracked_symphony(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
) -> StatusCode {
    let mut store = app_state.tracked_symphonies.lock().await;
    if store.symphonies.remove(&name).is_some() {
        app_state.symphony_tx.send(()).unwrap();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Build the router for tracked symphonies
pub fn tracked_symphonies_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_tracked_symphonies).post(create_tracked_symphony),
        )
        .route(
            "/{name}",
            put(update_tracked_symphony).delete(delete_tracked_symphony),
        )
}
