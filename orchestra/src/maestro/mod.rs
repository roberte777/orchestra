pub mod models;
pub mod repositories;
pub mod routes;

use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use models::{SharedStore, SharedStoreExt};
use repositories::note::{InMemoryNoteRepository, NoteRepository};

pub struct AppState {
    pub note_repository: Box<dyn NoteRepository>,
}

impl AppState {
    pub fn new(note_repository: Box<dyn NoteRepository>) -> Self {
        Self { note_repository }
    }
}

#[derive(Clone)]
pub struct SampleState {}

pub async fn maestro() {
    let data_store = SharedStore::new_shared();
    let note_repository = Box::new(InMemoryNoteRepository::new(data_store));
    let app_state = Arc::new(AppState::new(note_repository));
    let app = Router::new()
        .route(
            "/",
            get(|_: State<Arc<AppState>>| async move { "Hello World!" }),
        )
        .nest("/api/v1/notes", routes::note::routes())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
