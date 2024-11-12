use crate::maestro::models::{Note, SharedStore};
use async_trait::async_trait;

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn get_all_notes(&self) -> Vec<Note>;
    async fn get_note(&self, name: &str) -> Option<Note>;
    async fn add_note(&self, note: Note);
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
}
