use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::maestro::models::{DesiredState, Note, SharedStore};

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn get_all_notes(&self) -> Vec<Note>;
    async fn get_note(&self, name: &str) -> Option<Note>;
    async fn add_note(&self, note: Note);
    async fn update_note(&self, note: Note) -> bool;
    async fn remove_note(&self, name: &str) -> bool;

    async fn stop_note(&self, name: &str) -> bool;
    async fn start_note(&self, name: &str) -> bool;
}

pub struct InMemoryNoteRepository {
    pub store: SharedStore,
}

impl InMemoryNoteRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl NoteRepository for InMemoryNoteRepository {
    async fn get_all_notes(&self) -> Vec<Note> {
        let store = self.store.lock().await;
        store.get_all_notes()
    }

    async fn get_note(&self, name: &str) -> Option<Note> {
        let store = self.store.lock().await;
        store.get_note(name)
    }

    async fn add_note(&self, note: Note) {
        let mut store = self.store.lock().await;
        store.add_note(note);
    }

    async fn update_note(&self, note: Note) -> bool {
        let mut store = self.store.lock().await;
        store.update_note(note).is_ok()
    }

    async fn remove_note(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        store.remove_note(name).is_some()
    }

    async fn stop_note(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        if let Some(mut n) = store.get_note(name) {
            n.desired_state = DesiredState::Stop;
            return store.update_note(n).is_ok();
        }
        false
    }

    async fn start_note(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        if let Some(mut n) = store.get_note(name) {
            n.desired_state = DesiredState::Run;
            return store.update_note(n).is_ok();
        }
        false
    }
}
