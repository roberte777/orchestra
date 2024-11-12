pub mod models;
pub mod repositories;
pub mod routes;

use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use models::{Note, Principal, SharedStore, SharedStoreExt, Symphony};
use repositories::note::{InMemoryNoteRepository, NoteRepository};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use watcher::{Event, FieldSelector, Watcher};

pub struct AppState {
    pub note_repository: Mutex<Box<dyn NoteRepository>>,
    pub watch_manager: Mutex<WatchManager>,
}

impl AppState {
    pub fn new(note_repository: Box<dyn NoteRepository>, watch_manager: WatchManager) -> Self {
        Self {
            note_repository: Mutex::new(note_repository),
            watch_manager: Mutex::new(watch_manager),
        }
    }
}

#[derive(Clone)]
pub struct SampleState {}

pub async fn run_maestro() {
    let app = maestro();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
pub fn maestro() -> Router {
    let data_store = SharedStore::new_shared();
    let note_repository = Box::new(InMemoryNoteRepository::new(data_store));
    let watch_manager = WatchManager::default();
    let app_state = Arc::new(AppState::new(note_repository, watch_manager));
    Router::new()
        .route(
            "/",
            get(|_: State<Arc<AppState>>| async move { "Hello World!" }),
        )
        .nest("/api/v1/notes", routes::note::routes())
        .with_state(app_state)
}

pub struct WatchManager {
    note_watcher: Watcher<Note>,
    symphony_watcher: Watcher<Symphony>,
    principal_watcher: Watcher<Principal>,
}

impl WatchManager {
    pub fn new() -> Self {
        Self {
            note_watcher: Watcher::new(),
            symphony_watcher: Watcher::new(),
            principal_watcher: Watcher::new(),
        }
    }

    pub fn notify_note(&mut self, event: Event<Note>) {
        self.note_watcher.notify(event)
    }

    pub fn notify_symphony(&mut self, event: Event<Symphony>) {
        self.symphony_watcher.notify(event)
    }

    pub fn notify_principal(&mut self, event: Event<Principal>) {
        self.principal_watcher.notify(event)
    }

    pub fn subscribe_note(
        &mut self,
        field_selector: FieldSelector,
    ) -> UnboundedReceiver<Arc<Event<Note>>> {
        self.note_watcher.subscribe(field_selector)
    }

    pub fn subscribe_symphony(
        &mut self,
        field_selector: FieldSelector,
    ) -> UnboundedReceiver<Arc<Event<Symphony>>> {
        self.symphony_watcher.subscribe(field_selector)
    }

    pub fn subscribe_principal(
        &mut self,
        field_selector: FieldSelector,
    ) -> UnboundedReceiver<Arc<Event<Principal>>> {
        self.principal_watcher.subscribe(field_selector)
    }
}

impl Default for WatchManager {
    fn default() -> Self {
        Self::new()
    }
}
