use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    routing::get,
    Json, Router,
};

use futures::{stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::UnboundedReceiverStream;
use watcher::FieldSelector;

use crate::maestro::{models::Note, AppState};

#[derive(Deserialize)]
pub struct GetNotesQueryParams {
    watch: Option<bool>,
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
    let field_selector = FieldSelector::default(); // Customize if needed
    let receiver = app_state
        .watch_manager
        .lock()
        .await
        .subscribe_note(field_selector);

    // Stream of update events
    let update_stream = UnboundedReceiverStream::new(receiver).map(|event| {
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/notes", get(get_notes))
        .route("/notes/:name", get(get_notes))
}
