use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::KeepAlive, IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::{stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::debug;
use watcher::{Event, EventType, FieldSelector};

use crate::maestro::{
    dto::CreateSymphony,
    models::{Note, SymphonyState},
    AppState,
};

/// Create and start a Symphony
async fn start_symphony(
    State(app_state): State<Arc<AppState>>,
    Json(symphony_dto): Json<CreateSymphony>,
) -> impl IntoResponse {
    let symphony = symphony_dto.to_data_obj();
    let notes = symphony_dto
        .notes
        .iter()
        .map(|n| n.to_data_obj(&symphony.name))
        .collect::<Vec<Note>>();
    let symphony_repo = app_state.symphony_repository.lock().await;
    let notes_repo = app_state.note_repository.lock().await;

    // check for conflicts
    let symphony_name = symphony.name();
    // check if symphony exists
    if symphony_repo.get_symphony(&symphony_name).await.is_some() {
        return StatusCode::CONFLICT;
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
    let mut wm = app_state.watch_manager.lock().await;
    for note in notes {
        wm.notify_note(watcher::Event {
            event_type: watcher::EventType::Added,
            resource: note,
        });
    }

    let symphony = symphony_repo
        .get_symphony_with_notes(&symphony_name)
        .await
        .expect("Symphony must be defined");
    wm.notify_symphony(watcher::Event {
        event_type: watcher::EventType::Added,
        resource: symphony,
    });
    StatusCode::OK
}

// Stop a Symphony
async fn stop_symphony(
    Path(name): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let symphony_repo = app_state.symphony_repository.lock().await;
    let notes_repo = app_state.note_repository.lock().await;
    let symphony = match symphony_repo.get_symphony(&name).await {
        Some(symphony) => symphony,
        None => return StatusCode::NOT_FOUND,
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
    let symphony = symphony_repo
        .get_symphony_with_notes(&symphony.name)
        .await
        .expect("Symphony should not be able to be missing");
    wm.notify_symphony(watcher::Event {
        event_type: watcher::EventType::Modified,
        resource: symphony,
    });

    StatusCode::OK
}

async fn get_notes_for_symphony() {}

#[derive(Deserialize)]
struct GetSymphoniesQueryParams {
    watch: Option<bool>,
    field_selector: Option<String>,
}

async fn get_all_symphonies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GetSymphoniesQueryParams>,
) -> Response {
    let s_repo = state.symphony_repository.lock().await;
    let symphonies = s_repo.get_all_symphonies_with_notes().await;

    // Unified response and error handling for non-watching requests
    if !params.watch.unwrap_or(false) {
        return Json(symphonies).into_response();
    }

    // Create the initial SSE event with the list of symphonies to populate the client's cache
    let initial_event = axum::response::sse::Event::default()
        .event("list")
        .data(serde_json::to_string(&symphonies).unwrap_or_else(|_| "[]".to_string()));

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

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{symphony_name}/notes", get(get_notes_for_symphony))
        .route("/", post(start_symphony))
        .route("/", get(get_all_symphonies))
        .route("/{name}/stop", post(stop_symphony))
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use axum::http::StatusCode;
    use http_body_util::BodyExt;
    use tokio_stream::StreamExt;
    use tower::ServiceExt;
    use tracing_subscriber;

    use crate::{
        maestro::{
            dto::CreateSymphony,
            models::SymphonyState,
            repositories::{
                note::InMemoryNoteRepository, principal::InMemoryPrincipalRepository,
                symphony::InMemorySymphonyRepository,
            },
            AppState, WatchManager,
        },
        models::{SharedStore, SharedStoreExt},
    };
    fn create_state() -> Arc<AppState> {
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
    async fn test_start_symphony() {
        tracing_subscriber::fmt::try_init().ok();

        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let symphony_dto = CreateSymphony {
            name: "Test Symphony".into(),
            notes: vec![],
        };

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&symphony_dto).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let symphony_repo = app_state.symphony_repository.lock().await;
        let symphony = symphony_repo.get_symphony("Test Symphony").await;
        assert!(symphony.is_some());
        assert_eq!(*symphony.unwrap().state(), SymphonyState::Running);
    }

    #[tokio::test]
    async fn test_stop_symphony() {
        tracing_subscriber::fmt::try_init().ok();

        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let symphony_dto = CreateSymphony {
            name: "Test Symphony".into(),
            notes: vec![],
        };

        {
            let symphony_repo = app_state.symphony_repository.lock().await;
            symphony_repo.add_symphony(symphony_dto.to_data_obj()).await;
        }

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/Test%20Symphony/stop")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let symphony_repo = app_state.symphony_repository.lock().await;
        let symphony = symphony_repo.get_symphony("Test Symphony").await;
        assert!(symphony.is_some());
        assert_eq!(*symphony.unwrap().state(), SymphonyState::Terminating);
    }

    #[tokio::test]
    async fn test_get_all_symphonies() {
        tracing_subscriber::fmt::try_init().ok();

        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let symphony_dto = CreateSymphony {
            name: "Test Symphony".into(),
            notes: vec![],
        };

        {
            let symphony_repo = app_state.symphony_repository.lock().await;
            symphony_repo.add_symphony(symphony_dto.to_data_obj()).await;
        }

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

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let symphonies: Vec<CreateSymphony> = serde_json::from_slice(&body).unwrap();

        assert_eq!(symphonies.len(), 1);
        assert_eq!(symphonies[0].name, "Test Symphony");
    }

    #[tokio::test]
    async fn test_sse_get_all_symphonies() {
        tracing_subscriber::fmt::try_init().ok();

        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let symphony_dto = CreateSymphony {
            name: "Test Symphony".into(),
            notes: vec![],
        };

        {
            let symphony_repo = app_state.symphony_repository.lock().await;
            symphony_repo.add_symphony(symphony_dto.to_data_obj()).await;
        }

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/?watch=true")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Process the streaming response
        let mut body_stream = response.into_body().into_data_stream();
        let mut buffer = String::new();

        while let Some(chunk) = body_stream.next().await {
            let chunk = chunk.unwrap(); // Handle chunk errors appropriately
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            if buffer.contains("Test Symphony") {
                // Test passes as soon as the expected data is found
                return;
            }
        }

        // If the loop completes without finding the expected data, fail the test
        panic!("SSE stream did not contain 'Test Symphony'");
    }

    #[tokio::test]
    async fn test_start_existing_symphony() {
        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let symphony_dto = CreateSymphony {
            name: "Duplicate Symphony".into(),
            notes: vec![],
        };

        // Add the symphony first
        {
            let symphony_repo = app_state.symphony_repository.lock().await;
            symphony_repo.add_symphony(symphony_dto.to_data_obj()).await;
        }

        // Attempt to start the same symphony again
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&symphony_dto).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert that the request fails or returns the expected status code
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // Verify the symphony wasn't overwritten or started again
        let symphony_repo = app_state.symphony_repository.lock().await;
        let symphony = symphony_repo.get_symphony("Duplicate Symphony").await;
        assert!(symphony.is_some());
        assert_eq!(*symphony.unwrap().state(), SymphonyState::Running);
    }

    #[tokio::test]
    async fn test_stop_nonexistent_symphony() {
        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/Nonexistent%20Symphony/stop")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert that the response indicates failure
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    #[tokio::test]
    async fn test_start_symphony_invalid_json() {
        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let invalid_json = "{ invalid json }"; // Malformed JSON

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(invalid_json))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert that the response indicates a bad request
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn test_double_stop_symphony() {
        let app_state = create_state();
        let app = super::routes().with_state(app_state.clone());

        let symphony_dto = CreateSymphony {
            name: "Test Symphony".into(),
            notes: vec![],
        };

        {
            let symphony_repo = app_state.symphony_repository.lock().await;
            symphony_repo.add_symphony(symphony_dto.to_data_obj()).await;
            symphony_repo.stop_symphony("Test Symphony").await;
        }

        // Attempt to stop the symphony again
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/Test%20Symphony/stop")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert that the response is still OK
        assert_eq!(response.status(), StatusCode::OK);
    }
}
