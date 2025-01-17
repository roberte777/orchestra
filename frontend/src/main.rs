use axum::{
    body::Body,
    extract::Path,
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use include_dir::{include_dir, Dir};
use std::net::{IpAddr, SocketAddr};

#[cfg(feature = "frontend")]
static FRONTEND_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

pub async fn run_server(host: Option<String>, port: Option<u16>) {
    // build our application with a route
    let mut app = Router::new();

    #[cfg(feature = "frontend")]
    {
        app = app.route(
            "/",
            get(|| async {
                // Call `serve_embedded` with an empty string
                serve_embedded(Path("".to_owned())).await
            }),
        )
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
#[tokio::main]
async fn main() {
    let host = "localhost".to_string();
    let port = 8888;
    run_server(Some(host), Some(port)).await;
}
