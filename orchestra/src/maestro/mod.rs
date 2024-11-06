pub mod models;
pub mod repositories;
pub mod routes;

use axum::{routing::get, Router};

pub async fn maestro() {
    let app = Router::new()
        .route("/", get(|| async move { "Hello World!" }))
        .nest("/api/v1/notes", routes::note::routes());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
