use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::get,
    Json, Router,
};

use futures::StreamExt;
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
    // Unified response and error handling
    if !params.watch.unwrap_or(false) {
        // Early return for non-watching case
        let notes = app_state.note_repository.lock().await.get_all_notes().await;
        return Json(notes).into_response();
    }

    // Watching for updates
    let field_selector = FieldSelector::default(); // Can add customization here if needed
    let receiver = app_state
        .watch_manager
        .lock()
        .await
        .subscribe_note(field_selector);

    let stream = UnboundedReceiverStream::new(receiver).map(|event| {
        let note_event = event.as_ref().clone();
        serde_json::to_string(&note_event.resource).map(|data| {
            Event::default()
                .event(event.event_type.to_string())
                .data(data)
        })
    });

    Sse::new(stream).into_response()
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
