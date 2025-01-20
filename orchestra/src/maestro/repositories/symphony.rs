use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::maestro::models::{DesiredState, Note, SharedStore, Symphony};
use std::sync::Arc;

#[async_trait]
pub trait SymphonyRepository: Send + Sync {
    async fn get_all_symphonies(&self) -> Vec<Symphony>;
    async fn get_symphony(&self, name: &str) -> Option<Symphony>;
    /// Create a new Symphony along with its notes, all in one transaction.
    async fn add_symphony_with_notes(
        &self,
        symphony: Symphony,
        notes: Vec<Note>,
    ) -> Result<(), String>;

    /// Start a symphony by setting its `desired_state` to `Run`.
    async fn start_symphony(&self, name: &str) -> bool;
    /// Stop a symphony by setting its `desired_state` to `Stop`.
    async fn stop_symphony(&self, name: &str) -> bool;

    /// Remove just the symphony, no notes.
    async fn remove_symphony(&self, name: &str) -> bool;
    /// Remove the symphony and all of its notes in a single pass.
    async fn remove_symphony_and_notes(&self, name: &str) -> bool;

    /// Update an existing symphony object.
    async fn update_symphony(&self, symphony: Symphony) -> bool;

    /// Update a Symphony plus its notes in one transaction (optional).
    async fn update_symphony_with_notes(
        &self,
        symphony: Symphony,
        notes: Vec<Note>,
    ) -> Result<(), String>;
}

pub struct InMemorySymphonyRepository {
    pub store: SharedStore,
}

impl InMemorySymphonyRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl SymphonyRepository for InMemorySymphonyRepository {
    async fn get_all_symphonies(&self) -> Vec<Symphony> {
        let store = self.store.lock().await;
        store.get_all_symphonies()
    }

    async fn get_symphony(&self, name: &str) -> Option<Symphony> {
        let store = self.store.lock().await;
        store.get_symphony(name)
    }

    async fn add_symphony_with_notes(
        &self,
        symphony: Symphony,
        notes: Vec<Note>,
    ) -> Result<(), String> {
        let mut store = self.store.lock().await;
        store.add_symphony_with_notes(symphony, notes)
    }

    async fn start_symphony(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        match store.get_symphony(name) {
            Some(mut sym) => {
                sym.desired_state = DesiredState::Run;
                store.update_symphony(sym).is_ok()
            }
            None => false,
        }
    }

    async fn stop_symphony(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        match store.get_symphony(name) {
            Some(mut sym) => {
                sym.desired_state = DesiredState::Stop;
                store.update_symphony(sym).is_ok()
            }
            None => false,
        }
    }

    async fn remove_symphony(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        store.remove_symphony(name).is_some()
    }

    async fn remove_symphony_and_notes(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        store
            .remove_symphony_and_notes(name, false)
            .map(|_| true)
            .unwrap_or(false)
    }

    async fn update_symphony(&self, symphony: Symphony) -> bool {
        let mut store = self.store.lock().await;
        store.update_symphony(symphony).is_ok()
    }

    async fn update_symphony_with_notes(
        &self,
        symphony: Symphony,
        notes: Vec<Note>,
    ) -> Result<(), String> {
        let mut store = self.store.lock().await;
        store.update_symphony_with_notes(symphony, notes)
    }
}
