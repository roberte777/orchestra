pub mod cli;
pub mod config;
pub mod dto;
pub mod models;
pub(crate) mod repositories;
pub(crate) mod routes;

use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use config::MaestroConfig;
use dto::SymphonyWithNotes;
use models::{Note, Principal, SharedStore, SharedStoreExt};
use repositories::{
    note::{InMemoryNoteRepository, NoteRepository},
    principal::{InMemoryPrincipalRepository, PrincipalRepository},
    symphony::{InMemorySymphonyRepository, SymphonyRepository},
};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use watcher::{Event, FieldSelector, Watcher};

pub struct AppState {
    pub note_repository: Mutex<Box<dyn NoteRepository>>,
    pub symphony_repository: Mutex<Box<dyn SymphonyRepository>>,
    pub principal_repository: Mutex<Box<dyn PrincipalRepository>>,
    pub watch_manager: Mutex<WatchManager>,
}

impl AppState {
    pub fn new(
        note_repository: Box<dyn NoteRepository>,
        symphony_repository: Box<dyn SymphonyRepository>,
        principal_repository: Box<dyn PrincipalRepository>,
        watch_manager: WatchManager,
    ) -> Self {
        Self {
            note_repository: Mutex::new(note_repository),
            symphony_repository: Mutex::new(symphony_repository),
            principal_repository: Mutex::new(principal_repository),
            watch_manager: Mutex::new(watch_manager),
        }
    }
}

#[derive(Clone)]
pub struct SampleState {}

pub async fn run_maestro(config: MaestroConfig) -> anyhow::Result<()> {
    let app = maestro();
    let listener = tokio::net::TcpListener::bind(config.server_address)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
pub fn maestro() -> Router {
    let data_store = SharedStore::new_shared();
    let note_repository = Box::new(InMemoryNoteRepository::new(data_store.clone()));
    let symphony_repository = Box::new(InMemorySymphonyRepository::new(data_store.clone()));
    let principal_repository = Box::new(InMemoryPrincipalRepository::new(data_store.clone()));
    let watch_manager = WatchManager::default();
    let app_state = Arc::new(AppState::new(
        note_repository,
        symphony_repository,
        principal_repository,
        watch_manager,
    ));
    Router::new()
        .route(
            "/",
            get(|_: State<Arc<AppState>>| async move { "Hello World!" }),
        )
        .nest("/api/v1/notes", routes::notes::routes())
        .nest("/api/v1/symphonies", routes::symphonies::routes())
        .nest("/api/v1/principals", routes::principals::routes())
        .with_state(app_state)
}

pub struct WatchManager {
    note_watcher: Watcher<Note>,
    symphony_watcher: Watcher<SymphonyWithNotes>,
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

    pub fn notify_symphony(&mut self, event: Event<SymphonyWithNotes>) {
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
    ) -> UnboundedReceiver<Arc<Event<SymphonyWithNotes>>> {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use models::{NoteState, RestartPolicy};
    use watcher::EventType;

    use super::*;
    #[tokio::test]
    async fn test_notes_no_selector() {
        let mut watch_manager = WatchManager::default();
        let field_selector = FieldSelector::default();
        let mut receiver = watch_manager.subscribe_note(field_selector);
        let sample_note = Note {
            name: "Sample note".to_string(),
            description: "Sample desc".to_string(),
            host: "host".to_string(),
            command: "command".to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            restart_policy: RestartPolicy::Never,
            symphony: "sample".to_string(),
            state: NoteState::Pending,
        };

        let sample_event = Event {
            event_type: EventType::Added,
            resource: sample_note,
        };

        watch_manager.notify_note(sample_event.clone());
        let e = receiver.try_recv().expect("Should receive an event");
        assert!(sample_event.event_type.to_string() == e.event_type.to_string());
    }
}
