use std::{net::SocketAddr, time::Duration};

use axum::Router;
use orchestra::maestro::{maestro, models::NoteState};
use orchestra::{CreateNote, CreateSymphony, Note, RestartPolicy, SymphonyWithNotes};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tracing::info;
use watcher::EventType;

/// Spawns the Axum-based maestro server on an ephemeral port and returns:
/// - The [`SocketAddr`] (host+port) the server is bound to,
/// - A handle to the background server task that you can `await` or `abort` later.
async fn spawn_test_app() -> (SocketAddr, JoinHandle<()>) {
    // Build our maestro app as usual.
    let app: Router = maestro();

    // Bind to an ephemeral port on localhost:
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to ephemeral port");
    let addr = listener.local_addr().expect("Could not get local addr");

    // Spawn the Axum server in the background
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, server_handle)
}

#[tokio::test]
async fn test_create_and_list_symphonies() {
    let (addr, server_handle) = spawn_test_app().await;

    let client = reqwest::Client::new();

    // Create a new symphony with one note
    let symphony_body = CreateSymphony {
        name: "TestSymphony".to_string(),
        notes: vec![CreateNote {
            name: "NoteA".to_string(),
            description: "Test note A".to_string(),
            host: "localhost".to_string(),
            command: "echo".to_string(),
            args: vec!["Hello".into(), "World".into()],
            env: std::collections::HashMap::new(),
            restart_policy: RestartPolicy::Never,
        }],
    };

    // POST /api/v1/symphonies
    let res = client
        .post(format!("http://{}/api/v1/symphonies", addr))
        .json(&symphony_body)
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/v1/symphonies
    let res = client
        .get(format!("http://{}/api/v1/symphonies", addr))
        .send()
        .await
        .expect("Failed to fetch symphonies");
    assert_eq!(res.status(), StatusCode::OK);

    let symphonies: Vec<SymphonyWithNotes> = res
        .json()
        .await
        .expect("Failed to deserialize symphony list response");
    assert!(!symphonies.is_empty());

    let maybe_symphony = symphonies.iter().find(|s| s.name == "TestSymphony");
    assert!(maybe_symphony.is_some(), "Expected 'TestSymphony' to exist");

    let test_symphony = maybe_symphony.unwrap();
    assert_eq!(test_symphony.notes.len(), 1);
    assert_eq!(test_symphony.notes[0].name, "NoteA");
    assert_eq!(test_symphony.notes[0].state, NoteState::Pending);

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_stop_symphony() {
    let (addr, server_handle) = spawn_test_app().await;
    let client = reqwest::Client::new();

    // Create a new symphony with two notes
    let symphony_body = CreateSymphony {
        name: "StopMe".to_string(),
        notes: vec![
            CreateNote {
                name: "StopNote1".to_string(),
                description: "First note".to_string(),
                host: "localhost".to_string(),
                command: "echo".to_string(),
                args: vec!["Stopping".into()],
                env: std::collections::HashMap::new(),
                restart_policy: RestartPolicy::Never,
            },
            CreateNote {
                name: "StopNote2".to_string(),
                description: "Second note".to_string(),
                host: "localhost".to_string(),
                command: "ping".to_string(),
                args: vec!["127.0.0.1".into()],
                env: std::collections::HashMap::new(),
                restart_policy: RestartPolicy::Never,
            },
        ],
    };

    // Start it
    let res = client
        .post(format!("http://{}/api/v1/symphonies", addr))
        .json(&symphony_body)
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(res.status(), StatusCode::OK);

    // Stop it
    let res = client
        .post(format!("http://{}/api/v1/symphonies/StopMe/stop", addr))
        .send()
        .await
        .expect("Failed to send stop request");
    assert_eq!(res.status(), StatusCode::OK);

    // Verify the symphony is now in "Terminating" state
    // (Note that the examples in your code do not return the symphony’s updated state via GET,
    //  but we can at least confirm it still shows up. Implementation can vary.)
    let res = client
        .get(format!("http://{}/api/v1/symphonies", addr))
        .send()
        .await
        .expect("Failed to fetch symphonies after stopping");
    assert_eq!(res.status(), StatusCode::OK);
    let symphonies: Vec<SymphonyWithNotes> = res.json().await.unwrap();

    let maybe_stopped = symphonies.iter().find(|s| s.name == "StopMe");
    assert!(
        maybe_stopped.is_some(),
        "Expected to find 'StopMe' in the list of symphonies"
    );

    // Additional logic could be added if your application indicates the symphony state in GET results.
    // For now, we at least know we didn't error out on stop.
    server_handle.abort();
}

#[tokio::test]
async fn test_principal_heartbeat() {
    let (addr, server_handle) = spawn_test_app().await;
    let client = reqwest::Client::new();

    // There's no principal yet
    let res = client
        .get(format!("http://{}/api/v1/principals", addr))
        .send()
        .await
        .expect("Fetching principals failed");
    assert_eq!(res.status(), StatusCode::OK);
    let existing_principals: Vec<serde_json::Value> = res.json().await.unwrap();
    assert!(
        existing_principals.is_empty(),
        "Expected no principals initially"
    );

    // Post a principal heartbeat
    #[derive(Serialize)]
    struct HeartbeatNote {
        name: String,
        state: NoteState,
    }
    #[derive(Serialize)]
    struct HeartbeatDto {
        name: String,
        notes: Vec<HeartbeatNote>,
    }

    let payload = HeartbeatDto {
        name: "TestPrincipal".to_string(),
        notes: vec![HeartbeatNote {
            name: "someNote".to_string(),
            state: NoteState::Running,
        }],
    };

    let res = client
        .post(format!("http://{}/api/v1/principals", addr))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send heartbeat");
    assert_eq!(res.status(), StatusCode::OK);

    // Now we should see 1 principal
    let res = client
        .get(format!("http://{}/api/v1/principals", addr))
        .send()
        .await
        .expect("Fetching principals failed");
    assert_eq!(res.status(), StatusCode::OK);

    #[derive(Deserialize)]
    struct PrincipalResponse {
        host: String,
        // ignore other fields for brevity
    }
    let principals: Vec<PrincipalResponse> = res.json().await.unwrap();
    assert_eq!(principals.len(), 1);
    assert_eq!(principals[0].host, "TestPrincipal");

    server_handle.abort();
}

#[tokio::test]
async fn test_notes_sse_watch() {
    // Demonstrates subscribing to the SSE watch endpoint for notes, verifying the "list" event,
    // then creating a new symphony/note and verifying we get an "added" event.
    // This test involves async concurrency with SSE, so be aware of possible timing issues.

    let (addr, server_handle) = spawn_test_app().await;
    let client = reqwest::Client::new();

    // Start SSE subscription to /api/v1/notes?watch=true
    // For convenience, we'll use `reqwest_eventsource` or you can manually parse the SSE.
    use reqwest_eventsource::{Event as Evt, EventSource};

    let watch_url = format!("http://{}/api/v1/notes?watch=true", addr);
    let mut es = EventSource::get(&watch_url);

    // The first event we expect is a "list" event with existing notes (likely empty).
    let first_event = timeout(Duration::from_secs(5), es.next()).await;
    assert!(
        first_event.is_ok(),
        "Timed out waiting for SSE 'list' event"
    );
    let first_event = first_event.unwrap().expect("No SSE event received");
    match first_event {
        Ok(Evt::Open) => {
            // It's possible the first item is "Open", so let's read again
            let next = timeout(Duration::from_secs(5), es.next()).await;
            let next_evt = next.unwrap().expect("No SSE event after open");
            match next_evt {
                Ok(Evt::Message(msg)) => {
                    assert_eq!(msg.event.as_str(), "list", "Expected SSE list event");
                    info!("Received SSE 'list' event with data: {}", msg.data);
                }
                x => panic!("Expected SSE list event after open, got: {:?}", x),
            }
        }
        Ok(Evt::Message(msg)) => {
            assert_eq!(msg.event.as_str(), "list", "Expected SSE list event");
            info!("Received SSE 'list' event with data: {}", msg.data);
        }
        Err(e) => panic!("Error in SSE stream: {:?}", e),
    }

    // Now let's create a new symphony => which includes a note => which triggers an "added" event.
    let symphony_body = CreateSymphony {
        name: "WatchTestSymphony".to_string(),
        notes: vec![CreateNote {
            name: "WatchNote".to_string(),
            description: "Watch note desc".to_string(),
            host: "localhost".to_string(),
            command: "echo".to_string(),
            args: vec!["Hello".into()],
            env: std::collections::HashMap::new(),
            restart_policy: RestartPolicy::Never,
        }],
    };
    let res = client
        .post(format!("http://{}/api/v1/symphonies", addr))
        .json(&symphony_body)
        .send()
        .await
        .expect("Failed to send symphony creation request");
    assert_eq!(res.status(), StatusCode::OK);

    // Wait for the SSE "added" event:
    let added_event = timeout(Duration::from_secs(5), es.next()).await;
    assert!(
        added_event.is_ok(),
        "Timed out waiting for 'added' SSE event"
    );
    let added_event = added_event.unwrap().expect("Stream ended unexpectedly");
    match added_event {
        Ok(Evt::Message(msg)) => {
            assert_eq!(
                msg.event.as_str(),
                EventType::Added.to_string(),
                "Expected 'added' SSE event"
            );
            info!("Received SSE 'added' event with data: {}", msg.data);
            // We can optionally parse the note and check it matches "WatchNote"
            let note: Note =
                serde_json::from_str(&msg.data).expect("Failed to parse SSE note data");
            assert_eq!(note.name, "WatchNote");
            assert_eq!(note.symphony, "WatchTestSymphony");
        }
        _ => panic!("Expected SSE 'added' event"),
    }

    // Clean up SSE
    es.close();

    // Cleanup server
    server_handle.abort();
}
