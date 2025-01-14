use crate::maestro::models::{DesiredState, Note, SharedStore};
use async_trait::async_trait;

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn get_all_notes(&self) -> Vec<Note>;
    async fn get_note(&self, name: &str) -> Option<Note>;
    async fn add_note(&self, note: Note);
    async fn update_note(&self, note: Note);
    async fn stop_note(&self, name: &str) -> bool;
    async fn start_note(&self, name: &str) -> bool;
    async fn remove_note(&self, name: &str) -> bool;
}

pub struct InMemoryNoteRepository {
    store: SharedStore,
}

impl InMemoryNoteRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl NoteRepository for InMemoryNoteRepository {
    async fn get_all_notes(&self) -> Vec<Note> {
        self.store.lock().await.get_all_notes()
    }
    async fn get_note(&self, name: &str) -> Option<Note> {
        self.store.lock().await.get_note(name)
    }

    async fn add_note(&self, note: Note) {
        self.store.lock().await.add_note(note)
    }
    async fn update_note(&self, note: Note) {
        self.store
            .lock()
            .await
            .update_note(note)
            .expect("Should only be updating notes that exist");
    }

    async fn stop_note(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        let Some(mut note) = store.get_note(name) else {
            return false;
        };

        note.desired_state = DesiredState::Stop;
        store.update_note(note).is_ok()
    }
    async fn start_note(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        let Some(mut note) = store.get_note(name) else {
            return false;
        };

        note.desired_state = DesiredState::Run;
        store.update_note(note).is_ok()
    }
    async fn remove_note(&self, name: &str) -> bool {
        self.store.lock().await.remove_note(name);
        true
    }
}
