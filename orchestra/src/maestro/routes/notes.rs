use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    routing::{get, patch},
    Json, Router,
};
use futures::{stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::UnboundedReceiverStream;
use watcher::{EventType, FieldSelector};

use crate::maestro::{
    models::{DesiredState, Note},
    AppState,
};

#[derive(Deserialize)]
pub struct GetNotesQueryParams {
    watch: Option<bool>,
    field_selector: Option<String>,
}

pub async fn get_notes(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<GetNotesQueryParams>,
) -> Response {
    // Retrieve the initial list of notes
    let notes = app_state.note_repository.lock().await.get_all_notes().await;

    // If not a "watch" request, return the list as JSON
    if !params.watch.unwrap_or(false) {
        return Json(notes).into_response();
    }

    // Otherwise, we prepare an SSE stream
    // 1) The initial "list" event
    let initial_event = Event::default()
        .event("list")
        .data(serde_json::to_string(&notes).unwrap_or_else(|_| "[]".to_string()));

    // 2) Subscribe to watch updates via the WatchManager
    let field_selector =
        FieldSelector::from_query(&params.field_selector.unwrap_or("".to_string()));
    let receiver = app_state
        .watch_manager
        .lock()
        .await
        .subscribe_note(field_selector);

    // Convert the UnboundedReceiver into a stream of SSE events
    let update_stream = UnboundedReceiverStream::new(receiver).map(|event| {
        let cloned = event.as_ref().clone(); // watchers::Event<Note>
                                             // Convert the Note to JSON
        match serde_json::to_string(&cloned.resource) {
            Ok(data) => Ok(Event::default()
                .event(cloned.event_type.to_string())
                .data(data)),
            Err(e) => {
                let msg = format!("Failed to serialize note: {e}");
                Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
            }
        }
    });

    // Combine the initial event with the update stream
    let combined_stream = futures::stream::once(async { Ok::<_, std::io::Error>(initial_event) })
        .chain(update_stream);

    Sse::new(combined_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(1))
                .text("keep-alive"),
        )
        .into_response()
}

/// Get a single note by name
pub async fn get_note_by_name(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Note>, StatusCode> {
    match app_state.note_repository.lock().await.get_note(&name).await {
        Some(note) => Ok(Json(note)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Start (desired_state=Run) an existing note
pub async fn start_note(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> StatusCode {
    let repo = app_state.note_repository.lock().await;
    let existing_note = repo.get_note(&name).await;
    if existing_note.is_none() {
        return StatusCode::NOT_FOUND;
    }
    let note_obj = existing_note.unwrap();
    if matches!(note_obj.desired_state, DesiredState::Stop) {
        // If it was explicitly stopped, let’s re-start it
        let success = repo.start_note(&name).await;
        if success {
            // Fire watch event
            if let Some(updated_note) = repo.get_note(&name).await {
                let event = watcher::Event {
                    event_type: EventType::Modified,
                    resource: updated_note,
                };
                app_state.watch_manager.lock().await.notify_note(event);
            }
            return StatusCode::OK;
        } else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    // If it was already "run" or something else, we do nothing
    StatusCode::OK
}

/// Stop (desired_state=Stop) an existing note
pub async fn stop_note(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> StatusCode {
    let repo = app_state.note_repository.lock().await;
    let success = repo.stop_note(&name).await;
    if success {
        // Fire watch event
        if let Some(note) = repo.get_note(&name).await {
            let event = watcher::Event {
                event_type: EventType::Modified,
                resource: note,
            };
            app_state.watch_manager.lock().await.notify_note(event);
        }
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_notes))
        .route("/{name}", get(get_note_by_name))
        .route("/{name}/start", patch(start_note))
        .route("/{name}/stop", patch(stop_note))
}
