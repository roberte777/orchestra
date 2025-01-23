use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
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
use tracing::debug;
use watcher::FieldSelector;

use crate::maestro::AppState;

#[derive(Deserialize)]
struct GetNotesQueryParams {
    watch: Option<bool>,
    field_selector: Option<String>,
}

async fn get_notes(
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

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_notes))
}

#[cfg(test)]
mod test {
    use super::*;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::maestro::repositories::note::InMemoryNoteRepository;
    use crate::maestro::repositories::principal::InMemoryPrincipalRepository;
    use crate::maestro::repositories::symphony::InMemorySymphonyRepository;
    use crate::maestro::{AppState, WatchManager};
    use crate::models::{Note, SharedStore, SharedStoreExt};
    use crate::{NoteState, RestartPolicy};

    fn create_test_state() -> Arc<AppState> {
        let data_store = SharedStore::new_shared();
        let note_repository = Box::new(InMemoryNoteRepository::new(data_store.clone()));
        let symphony_repository = Box::new(InMemorySymphonyRepository::new(data_store.clone()));
        let principal_repository = Box::new(InMemoryPrincipalRepository::new(data_store.clone()));
        let watch_manager = WatchManager::default();

        Arc::new(AppState::new(
            note_repository,
            symphony_repository,
            principal_repository,
            watch_manager,
        ))
    }

    #[tokio::test]
    async fn test_get_notes_non_watching() {
        let app_state = create_test_state();
        let app = routes().with_state(app_state.clone());

        // Add mock notes
        {
            let note_repo = app_state.note_repository.lock().await;
            note_repo
                .add_note(Note {
                    name: "Note 1".into(),
                    symphony: "Symphony 1".into(),
                    description: "desc".into(),
                    host: "host".into(),
                    command: "ping".into(),
                    env: HashMap::default(),
                    args: Vec::new(),
                    restart_policy: RestartPolicy::Never,
                    state: NoteState::Pending,
                })
                .await;
            note_repo
                .add_note(Note {
                    name: "Note 2".into(),
                    symphony: "Symphony 2".into(),
                    description: "desc".into(),
                    host: "host".into(),
                    command: "ping".into(),
                    env: HashMap::default(),
                    args: Vec::new(),
                    restart_policy: RestartPolicy::Never,
                    state: NoteState::Pending,
                })
                .await;
        }

        // Perform a GET request without watching
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert the response status and body
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let notes: Vec<Note> = serde_json::from_slice(&body).unwrap();

        assert_eq!(notes.len(), 2);
        assert!(notes.iter().any(|note| note.name == "Note 1"));
        assert!(notes.iter().any(|note| note.name == "Note 2"));
    }
}
