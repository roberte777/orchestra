use auditorium::{
    tracked_symphonies::{tracked_symphonies_routes, InMemoryAuditoriumStore},
    AppState,
};
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::{any, get, post},
    Json, Router,
};
use include_dir::{include_dir, Dir};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::sync::{
    broadcast::{self, Receiver},
    Mutex,
};

#[cfg(feature = "frontend")]
static FRONTEND_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

pub async fn run_server(maestro_url: String, host: Option<String>, port: Option<u16>) {
    let client = Client::new();
    let (symphony_tx, _) = broadcast::channel::<String>(100);

    // TODO: This will use SSE in the future instead of polling itself for updates
    tokio::spawn({
        let symphony_tx = symphony_tx.clone();

        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let response = reqwest::get(format!(
                    "{}/api/v1/tracked-symphonies",
                    "http://localhost:8888"
                ))
                .await;

                if let Ok(resp) = response {
                    match resp.text().await {
                        Ok(txt) => {
                            let _ = symphony_tx.send(txt);
                        }
                        Err(e) => {
                            eprintln!(
                                "Malformed data received from tracked symphonies endpoint: {}",
                                e
                            );
                        }
                    }
                } else {
                    println!("Failed to make request to retrieve tracked symphonies");
                }
            }
        }
    });

    let app_state = AppState {
        maestro_url,
        client,
        tracked_symphonies: Arc::new(Mutex::new(InMemoryAuditoriumStore::with_defaults())),
        symphony_tx,
    };
    // build our application with a route
    let mut app = Router::new()
        .route("/ws", any(ws_handler))
        .route("/api/v1/symphonies", post(start_symphony))
        .route("/api/v1/symphonies/{name}/stop", post(stop_symphony))
        .nest("/api/v1/tracked-symphonies", tracked_symphonies_routes())
        .with_state(app_state);

    #[cfg(feature = "frontend")]
    {
        app = app
            .route(
                "/",
                get(|| async {
                    // Call `serve_embedded` with an empty string
                    serve_embedded(Path("".to_owned())).await
                }),
            )
            .route("/{path}", get(|path| async { serve_embedded(path).await }))
    }

    let ip: IpAddr = match host {
        Some(address) => address.parse().expect("Failed to parse IP address"),
        None => "127.0.0.1".to_string().parse().unwrap(),
    };

    let addr = SocketAddr::new(ip, port.unwrap_or(3000));
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to port");
    axum::serve(listener, app).await.unwrap();
}

#[cfg(feature = "frontend")]
async fn serve_embedded(Path(req_path): Path<String>) -> impl IntoResponse {
    // If path is empty, serve "index.html"

    let req_path = if req_path.trim().is_empty() {
        "index.html"
    } else {
        req_path.as_str()
    };

    match FRONTEND_DIST.get_file(req_path) {
        Some(file) => {
            let mime_type = mime_guess::from_path(req_path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_type.as_ref())
                .body(Body::from(file.contents()))
                .unwrap()
        }
        None => {
            // File not found in the embed — fall back to index.html
            match FRONTEND_DIST.get_file("index.html") {
                Some(file) => {
                    let mime_type = mime_guess::from_path("index.html").first_or_octet_stream();
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, mime_type.as_ref())
                        .body(Body::from(file.contents()))
                        .unwrap()
                }
                None => {
                    // If even index.html doesn't exist, return 404
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("404 Not Found"))
                        .unwrap()
                }
            }
        }
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state.symphony_tx.subscribe()))
}

async fn handle_websocket(mut socket: WebSocket, mut symphony_rx: Receiver<String>) {
    println!("New WebSocket connection");

    // Spawn tracked symphony sender task
    tokio::spawn(async move {
        loop {
            // TODO: Add cancellation token so we can stop this task later if needed
            tokio::select! {
                msg = symphony_rx.recv() => {
                    if let Ok(msg) = msg {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            println!("Client disconnected");
                            break;
                        }
                    } else {
                        println!("Failed to receive message from symphony_rx channel");
                        break;
                    }
                },
            }
        }
    });

    println!("WebSocket connection closed");
}

/// Create and start a Symphony
pub async fn start_symphony(
    State(app_state): State<AppState>,
    Json(symphony_dto): Json<SymphonyDto>,
) -> impl IntoResponse {
    let resp = app_state
        .client
        .post(format!("{}/api/v1/symphonies", app_state.maestro_url))
        .json(&symphony_dto)
        .send()
        .await;
    match resp {
        Ok(resp) => (resp.status(), resp.text().await.unwrap()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to make request to maestro server".to_string(),
        ),
    }
}

pub async fn stop_symphony(
    Path(name): Path<String>,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    let resp = app_state
        .client
        .post(format!(
            "{}/api/v1/symphonies/{}/stop",
            app_state.maestro_url, name
        ))
        .send()
        .await;
    match resp {
        Ok(resp) => (resp.status(), resp.text().await.unwrap()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to make request to maestro server".to_string(),
        ),
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct NoteDto {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SymphonyDto {
    pub name: String,
    pub notes: Vec<NoteDto>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
}

#[tokio::main]
async fn main() {
    let host = "127.0.0.1".to_string();
    let port = 8888;
    let maestro_url = "http://localhost:3000".to_string();
    run_server(maestro_url, Some(host), Some(port)).await;
}
