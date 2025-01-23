use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use tracing::{debug, warn};
use watcher::EventType;

use crate::maestro::{
    dto::HeartbeatDto,
    models::{NoteState, Principal, PrincipalState, SymphonyState},
    AppState,
};

async fn principal_heartbeat(
    State(app_state): State<Arc<AppState>>,
    Json(heartbeat): Json<HeartbeatDto>,
) {
    debug!("got heartbeat: {:?}", heartbeat);
    let principal_repo = app_state.principal_repository.lock().await;
    let note_repo = app_state.note_repository.lock().await;
    let symphony_repo = app_state.symphony_repository.lock().await;

    for note in heartbeat.notes {
        // remove terminated notes from state
        if matches!(note.state, NoteState::Terminated) {
            // collect data
            let note = match note_repo.get_note(&note.symphony, &note.name).await {
                Some(note) => note,
                None => continue,
            };
            let symphony_name = note.symphony.clone();
            let mut symphony = symphony_repo.get_symphony(&symphony_name).await.unwrap();

            // update symphony
            symphony.notes.retain(|n| *n != note.name);
            let should_remove = matches!(symphony.state(), SymphonyState::Terminating)
                && symphony.notes().is_empty();
            _ = symphony_repo.update_symphony(symphony.clone()).await;
            //update note
            note_repo.remove_note(&note.symphony, &note.name).await;
            let event = watcher::Event {
                event_type: EventType::Deleted,
                resource: note,
            };
            app_state.watch_manager.lock().await.notify_note(event);

            // remove symphony if stop requested and all notes gone
            if should_remove {
                let symphony = symphony_repo
                    .get_symphony_with_notes(&symphony_name)
                    .await
                    .expect("Symphony should not be able to be missing");
                symphony_repo.remove_symphony(&symphony_name).await;
                let event = watcher::Event {
                    event_type: EventType::Deleted,
                    resource: symphony,
                };
                app_state.watch_manager.lock().await.notify_symphony(event);
            }
        } else {
            let mut note_update = match note_repo.get_note(&note.symphony, &note.name).await {
                Some(n) => n,
                None => {
                    warn!(
                        principal = heartbeat.name,
                        "Principal out of sync with server"
                    );
                    continue;
                }
            };

            // terminating notes can only be updated to terminated
            if matches!(note_update.state, NoteState::Terminating) {
                continue;
            }

            if note_update.state != note.state {
                note_update.state = note.state;
                note_repo.update_note(note_update.clone()).await;
                let event = watcher::Event {
                    event_type: EventType::Modified,
                    resource: note_update.clone(),
                };
                app_state.watch_manager.lock().await.notify_note(event);
                let symphony = symphony_repo
                    .get_symphony_with_notes(&note_update.symphony)
                    .await
                    .unwrap();
                let event = watcher::Event {
                    event_type: EventType::Modified,
                    resource: symphony,
                };
                app_state.watch_manager.lock().await.notify_symphony(event);
            }
        }
    }

    let principal = Principal {
        host: heartbeat.name,
        capabilities: Vec::new(),
        state: PrincipalState::Ready,
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Should be able to get a valid duration since unix epoch")
            .as_secs(),
    };

    principal_repo.upsert_principal(principal).await;
}

async fn get_principals(State(app_state): State<Arc<AppState>>) -> Json<Vec<Principal>> {
    let principal_repo = app_state.principal_repository.lock().await;
    let principals = principal_repo.get_all_principals().await;
    Json(principals)
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(principal_heartbeat))
        .route("/", get(get_principals))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::{SharedStore, SharedStoreExt};
    use crate::{
        maestro::{
            models::{InMemoryStore, Note, NoteState, Principal, PrincipalState, RestartPolicy},
            repositories::{
                note::{InMemoryNoteRepository, NoteRepository},
                principal::{InMemoryPrincipalRepository, PrincipalRepository},
                symphony::{InMemorySymphonyRepository, SymphonyRepository},
            },
            AppState, WatchManager,
        },
        Symphony,
    };
    use axum::{http::StatusCode, Router};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt; // For `app.oneshot(...)`

    /// Helper to create a fresh in-memory state and attach the principal routes.
    fn create_test_app() -> (Router, Arc<AppState>) {
        // Create the shared store
        let data_store: SharedStore = SharedStore::new_shared();
        {
            // You can pre-populate data here if you want to test "already existing" principals or notes:
            // let mut store = data_store.blocking_lock();
            // e.g. store.add_principal(Principal { ... });
            // e.g. store.add_note(Note { ... });
        }

        // Create the repositories
        let note_repo = Box::new(InMemoryNoteRepository::new(data_store.clone()));
        let symphony_repo = Box::new(InMemorySymphonyRepository::new(data_store.clone()));
        let principal_repo = Box::new(InMemoryPrincipalRepository::new(data_store.clone()));
        let watch_manager = WatchManager::default();

        // Build the AppState
        let app_state = Arc::new(AppState::new(
            note_repo,
            symphony_repo,
            principal_repo,
            watch_manager,
        ));

        // Create the Axum router with these principal routes
        let router = routes().with_state(app_state.clone());
        (router, app_state)
    }

    #[tokio::test]
    async fn test_get_principals_empty() {
        let (app, _state) = create_test_app();
        // No principals added initially

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

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let principals: Vec<Principal> = serde_json::from_slice(&body_bytes).unwrap();
        assert!(principals.is_empty(), "Expected no principals initially");
    }

    #[tokio::test]
    async fn test_get_principals_with_existing() {
        let (app, state) = create_test_app();
        // Manually insert a principal
        {
            let principal_repo = state.principal_repository.lock().await;
            principal_repo
                .upsert_principal(Principal {
                    host: "some-host".to_string(),
                    capabilities: vec!["test-cap".to_string()],
                    state: PrincipalState::NotReady,
                    last_updated: 12345,
                })
                .await;
        }

        let response = app
            .clone()
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

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let principals: Vec<Principal> = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(principals.len(), 1, "Expected one principal in the list");
        assert_eq!(principals[0].host, "some-host");
        assert!(matches!(principals[0].state, PrincipalState::NotReady));
    }

    #[tokio::test]
    async fn test_post_heartbeat_new_principal() {
        let (app, state) = create_test_app();

        // Prepare a heartbeat JSON for a brand new principal
        let payload = json!({
            "name": "brand-new-host",
            "notes": [] // no notes
        });

        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // The principal_heartbeat handler doesn't return a body, so we check only status
        assert_eq!(response.status(), StatusCode::OK);

        // Verify the principal is inserted/upserted
        let principal_repo = state.principal_repository.lock().await;
        let principals = principal_repo.get_all_principals().await;
        assert_eq!(principals.len(), 1);
        assert_eq!(principals[0].host, "brand-new-host");
        // The code sets `state = PrincipalState::Ready` on heartbeat
        assert!(matches!(principals[0].state, PrincipalState::Ready));
    }

    #[tokio::test]
    async fn test_post_heartbeat_existing_principal() {
        let (app, state) = create_test_app();

        // Insert a principal first
        {
            let principal_repo = state.principal_repository.lock().await;
            principal_repo
                .upsert_principal(Principal {
                    host: "some-host".to_string(),
                    capabilities: vec!["test-cap".into()],
                    state: PrincipalState::NotReady,
                    last_updated: 100,
                })
                .await;
        }

        // Send a heartbeat for the same principal
        let payload = json!({
            "name": "some-host", // same principal
            "notes": []
        });
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check that the principal's last_updated changed and state became Ready
        let principal_repo = state.principal_repository.lock().await;
        let principals = principal_repo.get_all_principals().await;
        assert_eq!(principals.len(), 1, "Expected one principal");
        assert_eq!(principals[0].host, "some-host");
        assert!(matches!(principals[0].state, PrincipalState::Ready));
        // last_updated is set to the current system time in seconds, so we just
        // assert it is > 100 (the original)
        assert!(
            principals[0].last_updated > 100,
            "Expected last_updated to be updated"
        );
    }

    #[tokio::test]
    async fn test_post_heartbeat_with_known_notes() {
        let (app, state) = create_test_app();

        // Add a note so that the principal_heartbeat can update it
        {
            let note = Note {
                name: "existing-note".to_string(),
                symphony: "my-symphony".to_string(),
                description: "some desc".into(),
                host: "brand-new-host".into(),
                command: "ping".into(),
                args: vec![],
                env: HashMap::new(),
                restart_policy: RestartPolicy::Never,
                state: NoteState::Running,
            };
            let symphony_repo = state.symphony_repository.lock().await;
            symphony_repo
                .add_symphony(Symphony {
                    state: SymphonyState::Running,
                    name: "my-symphony".into(),
                    notes: vec![note.name.clone()],
                })
                .await;
            let note_repo = state.note_repository.lock().await;
            note_repo.add_note(note).await;
        }

        // Heartbeat with the same note set to Terminated
        let payload = json!({
            "name": "brand-new-host",
            "notes": [
                {
                    "symphony": "my-symphony",
                    "name": "existing-note",
                    "state": "Terminated"
                }
            ]
        });

        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // The note is "Terminated" => code checks for removing it from store
        // if "Terminated" is fully processed
        let note_repo = state.note_repository.lock().await;
        let found_note = note_repo.get_note("my-symphony", "existing-note").await;
        assert!(
            found_note.is_none(),
            "The route logic (since the note is Terminated) should remove the note"
        );
    }

    #[tokio::test]
    async fn test_post_heartbeat_with_unknown_note() {
        let (app, state) = create_test_app();
        // No notes or principal in the store for "some-other-note"

        let payload = json!({
            "name": "host-with-unknown-note",
            "notes": [
                {
                    "symphony": "my-symphony",
                    "name": "some-other-note",
                    "state": "Running"
                }
            ]
        });

        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The code logs a warning about "Principal out of sync with server" but otherwise
        // does not create or remove that note. Check that no note got inserted:
        let note_repo = state.note_repository.lock().await;
        let all_notes = note_repo.get_all_notes().await;
        assert!(
            all_notes.is_empty(),
            "The unknown note should not have been created/inserted"
        );

        // The principal is at least upserted
        let principal_repo = state.principal_repository.lock().await;
        let principals = principal_repo.get_all_principals().await;
        assert_eq!(principals.len(), 1);
        assert_eq!(principals[0].host, "host-with-unknown-note");
    }

    #[tokio::test]
    async fn test_post_heartbeat_modifies_note_state() {
        let (app, state) = create_test_app();

        // Insert a note with state = Pending
        {
            let note = Note {
                name: "note-to-update".to_string(),
                symphony: "my-symphony".to_string(),
                description: "some desc".into(),
                host: "my-host".into(),
                command: "ping".into(),
                args: vec![],
                env: HashMap::new(),
                restart_policy: RestartPolicy::Never,
                state: NoteState::Pending,
            };
            let symphony_repo = state.symphony_repository.lock().await;
            symphony_repo
                .add_symphony(Symphony {
                    state: SymphonyState::Running,
                    name: "my-symphony".into(),
                    notes: vec![note.name.clone()],
                })
                .await;
            let note_repo = state.note_repository.lock().await;
            note_repo.add_note(note).await;
        }

        let payload = json!({
            "name": "my-host",
            "notes": [
                {
                    "symphony": "my-symphony",
                    "name": "note-to-update",
                    "state": "Running" // changes from Pending -> Running
                }
            ]
        });

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify note's state is updated
        let note_repo = state.note_repository.lock().await;
        let updated_note = note_repo
            .get_note("my-symphony", "note-to-update")
            .await
            .expect("Should exist");
        assert_eq!(updated_note.state, NoteState::Running);
    }

    #[tokio::test]
    async fn test_post_heartbeat_invalid_json() {
        let (app, _state) = create_test_app();

        // This is invalid JSON
        let invalid_payload = "{ invalid json }";

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(invalid_payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum will return BAD_REQUEST since it cannot parse the JSON into `HeartbeatDto`.
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
