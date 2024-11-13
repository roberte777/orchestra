use std::sync::Arc;

use axum::{routing::get, Router};

use crate::maestro::AppState;

pub async fn get_notes_for_symphony() {}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/symphonies/:symphony_name/notes",
        get(get_notes_for_symphony),
    )
}
