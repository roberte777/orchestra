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
use tracing::debug;
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

    // Unified response and error handling for non-watching requests
    if !params.watch.unwrap_or(false) {
        return Json(notes).into_response();
    }

    // Create the initial SSE event with the list of notes to populate the client's cache
    let initial_event = Event::default()
        .event("list")
        .data(serde_json::to_string(&notes).unwrap_or_else(|_| "[]".to_string()));

    // Watching for updates
    let field_selector =
        FieldSelector::from_query(&params.field_selector.unwrap_or("".to_string())); // Customize if needed
    let receiver = app_state
        .watch_manager
        .lock()
        .await
        .subscribe_note(field_selector);

    // Stream of update events
    let update_stream = UnboundedReceiverStream::new(receiver).map(|event| {
        debug!("Got new event!");
        let note_event = event.as_ref().clone();
        serde_json::to_string(&note_event.resource).map(|data| {
            Event::default()
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

pub async fn get_note_by_name(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Note>, StatusCode> {
    match app_state.note_repository.lock().await.get_note(&name).await {
        Some(note) => Ok(Json(note)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn start_note(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> StatusCode {
    let repository = app_state.note_repository.lock().await;
    // we can only restart a note if its desired state is run (i.e. it has not
    // been terminated by a user yet.)

    let existing_note = match repository.get_note(&name).await {
        Some(note) => note,
        None => return StatusCode::NOT_FOUND,
    };

    // can't start a stopped process
    if matches!(existing_note.desired_state, DesiredState::Stop) {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    match repository.start_note(&name).await {
        true => {
            let note = repository.get_note(&name).await.unwrap();
            let event = watcher::Event {
                event_type: EventType::Modified,
                resource: note,
            };
            app_state.watch_manager.lock().await.notify_note(event);
            StatusCode::OK
        }
        false => StatusCode::NOT_FOUND,
    }
}

pub async fn stop_note(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> StatusCode {
    let repository = app_state.note_repository.lock().await;
    match repository.stop_note(&name).await {
        true => {
            let note = repository.get_note(&name).await.unwrap();
            let event = watcher::Event {
                event_type: EventType::Modified,
                resource: note,
            };
            app_state.watch_manager.lock().await.notify_note(event);

            StatusCode::OK
        }
        false => StatusCode::NOT_FOUND,
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_notes))
        .route("/{name}", get(get_note_by_name))
        .route("/{name}/start", patch(start_note))
        .route("/{name}/stop", patch(stop_note))
}
