use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};

use crate::maestro::{models::Note, AppState};

pub async fn get_notes(State(app_state): State<Arc<AppState>>) -> Json<Vec<Note>> {
    Json(app_state.note_repository.get_all_notes().await)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/notes", get(get_notes))
}
