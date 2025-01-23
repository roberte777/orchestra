use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::KeepAlive, IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::debug;
use watcher::{Event, EventType, FieldSelector};

use crate::maestro::{
    models::{Note, NoteState, RestartPolicy, Symphony, SymphonyState},
    AppState,
};

/// Create and start a Symphony
pub async fn start_symphony(
    State(app_state): State<Arc<AppState>>,
    Json(symphony_dto): Json<SymphonyDto>,
) {
    let symphony = symphony_dto.to_data_obj();
    let notes = symphony_dto
        .notes
        .iter()
        .map(|n| n.to_data_obj(&symphony.name))
        .collect::<Vec<Note>>();
    {
        let symphony_repo = app_state.symphony_repository.lock().await;
        let notes_repo = app_state.note_repository.lock().await;
        let symphony_name = symphony.name();
        // check if symphony exists
        if symphony_repo.get_symphony(&symphony_name).await.is_some() {
            return;
        }
        // Create and start symphony
        symphony_repo.add_symphony(symphony.clone()).await;
        symphony_repo.start_symphony(&symphony_name).await;
        // Create and start notes in the symphony
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
    wm.notify_symphony(watcher::Event {
        event_type: watcher::EventType::Added,
        resource: symphony,
    });
}

// Stop a Symphony
pub async fn stop_symphony(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let symphony_repo = app_state.symphony_repository.lock().await;
    let notes_repo = app_state.note_repository.lock().await;
    let symphony = match symphony_repo.get_symphony(&name).await {
        Some(symphony) => symphony,
        None => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    // if already stopping, return OK with no work
    if matches!(symphony.state(), SymphonyState::Terminating) {
        return StatusCode::OK;
    }
    for note in symphony.notes() {
        let success = notes_repo.stop_note(&note).await;
        if success {
            let note = notes_repo.get_note(&note).await.unwrap();
            let event = Event {
                event_type: EventType::Modified,
                resource: note,
            };
            app_state.watch_manager.lock().await.notify_note(event);
        } else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    symphony_repo.stop_symphony(&name).await;

    let mut wm = app_state.watch_manager.lock().await;
    wm.notify_symphony(watcher::Event {
        event_type: watcher::EventType::Modified,
        resource: symphony,
    });

    StatusCode::OK
}

pub async fn get_notes_for_symphony() {}

#[derive(Deserialize)]
pub struct GetSymphoniesQueryParams {
    watch: Option<bool>,
    field_selector: Option<String>,
}
pub async fn get_all_symphonies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GetSymphoniesQueryParams>,
) -> Response {
    let s_repo = state.symphony_repository.lock().await;
    let n_repo = state.note_repository.lock().await;
    let symphonies = s_repo.get_all_symphonies().await;

    let mut final_symphonies = Vec::new();
    for symphony in symphonies {
        let mut notes = Vec::new();
        for note in symphony.notes {
            let note = n_repo.get_note(&note).await.unwrap();
            notes.push(note);
        }

        final_symphonies.push(SymphonyReturn {
            name: symphony.name,
            notes,
        });
    }

    // Unified response and error handling for non-watching requests
    if !params.watch.unwrap_or(false) {
        return Json(final_symphonies).into_response();
    }

    // Create the initial SSE event with the list of symphonies to populate the client's cache
    let initial_event = axum::response::sse::Event::default()
        .event("list")
        .data(serde_json::to_string(&final_symphonies).unwrap_or_else(|_| "[]".to_string()));

    // Watching for updates
    let field_selector =
        FieldSelector::from_query(&params.field_selector.unwrap_or("".to_string())); // Customize if needed
    let receiver = state
        .watch_manager
        .lock()
        .await
        .subscribe_symphony(field_selector);

    // Stream of update events
    let update_stream = UnboundedReceiverStream::new(receiver).map(|event| {
        debug!("Got new event!");
        let symphony_event = event.as_ref().clone();
        serde_json::to_string(&symphony_event.resource).map(|data| {
            axum::response::sse::Event::default()
                .event(event.event_type.to_string())
                .data(data)
        })
    });

    // Combine the initial event with the update stream
    let combined_stream = stream::once(async { Ok::<_, _>(initial_event) }).chain(update_stream);

    Sse::new(combined_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(1))
                .text("keep-alive"),
        )
        .into_response()
}

#[derive(Serialize)]
struct SymphonyReturn {
    name: String,
    notes: Vec<Note>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{symphony_name}/notes", get(get_notes_for_symphony))
        .route("/", post(start_symphony))
        .route("/", get(get_all_symphonies))
        .route("/{name}/stop", post(stop_symphony))
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
            env: self.env.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            restart_policy: self.restart_policy.clone(),
            state: NoteState::Pending,
            symphony: symphony_name.to_string(),
        }
    }
}

#[derive(Clone, Deserialize)]
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
            state: SymphonyState::Running,
        }
    }
}
