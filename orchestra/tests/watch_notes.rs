use axum::{routing::get, Router};
use core::panic;
use orchestra::maestro::{
    models::{DesiredState, Note, NoteState, SharedStore, SharedStoreExt},
    repositories::note::InMemoryNoteRepository,
    routes::note::get_notes,
    AppState, WatchManager,
};
use reqwest::StatusCode;
use reqwest_eventsource::EventSource;
use std::{collections::HashMap, sync::Arc};
use tokio_stream::StreamExt;
use watcher::{Event, EventType};

// Integration test for `get_notes` function
#[tokio::test]
async fn test_get_notes() {
    // Setup app state with mocked repository and manager
    let shared_store = SharedStore::new_shared();
    let note_repository = Box::new(InMemoryNoteRepository::new(shared_store));
    let watch_manager = WatchManager::default();
    let app_state = Arc::new(AppState::new(note_repository, watch_manager));

    // Define the route with the `get_notes` function

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tokio::spawn({
        let app_state = Arc::clone(&app_state);
        async move {
            let app = Router::new()
                .route("/notes", get(get_notes))
                .with_state(app_state);
            axum::serve(listener, app).await.unwrap();
        }
    });

    // Test the non-watching case (`watch` parameter is `false` or not present)
    let client = reqwest::Client::new();
    let res = client
        .get("http://localhost:3000/notes?watch=false")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let notes: Vec<Note> = res.json().await.expect("Failed to parse response");
    assert_eq!(notes.len(), 0);

    // Test the watching case (`watch` parameter is `true`)
    let mut es = EventSource::get("http://localhost:3000/notes?watch=true");

    // wait for the connection to open.
    es.next().await;

    let sample_note = Note {
        name: "Sample note".to_string(),
        description: "Sample desc".to_string(),
        host: "host".to_string(),
        command: "command".to_string(),
        args: Vec::new(),
        env: HashMap::new(),
        restart_policy: "never".to_string(),
        symphony: "sample".to_string(),
        state: NoteState::Pending,
        desired_state: DesiredState::Run,
    };

    let sample_event = Event {
        event_type: EventType::Added,
        resource: sample_note,
    };

    app_state
        .watch_manager
        .lock()
        .await
        .notify_note(sample_event);

    let expected_events = 2;
    let mut received_events = 0;

    // Check Server-Sent Events stream response
    while let Some(item) = es.next().await {
        match item {
            // should never get because of the previous es.next().await
            Ok(reqwest_eventsource::Event::Open) => println!("Connection Opened!"),
            Ok(reqwest_eventsource::Event::Message(message)) => match message.event.as_str() {
                "list" => {
                    received_events += 1;
                }
                "added" => {
                    received_events += 1;
                }
                "modified" => {}
                "deleted" => {}
                _ => {
                    eprintln!("Unexpected message: {:#?}", message);
                }
            },
            Err(err) => {
                es.close();
                panic!("Event source error: {:#?}", err)
            }
        }
        if expected_events == received_events {
            break;
        }
    }
    assert!(received_events > 0, "Expected at least one event");
    es.close();
}
