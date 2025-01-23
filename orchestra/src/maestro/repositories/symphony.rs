use crate::maestro::{
    dto::SymphonyWithNotes,
    models::{SharedStore, Symphony, SymphonyState},
};
use async_trait::async_trait;

#[async_trait]
pub trait SymphonyRepository: Send + Sync {
    async fn get_all_symphonies(&self) -> Vec<Symphony>;
    async fn get_all_symphonies_with_notes(&self) -> Vec<SymphonyWithNotes>;
    async fn get_symphony(&self, name: &str) -> Option<Symphony>;
    async fn get_symphony_with_notes(&self, name: &str) -> Option<SymphonyWithNotes>;
    async fn add_symphony(&self, symphony: Symphony);
    async fn stop_symphony(&self, name: &str) -> bool;
    async fn remove_symphony(&self, name: &str) -> bool;
    async fn start_symphony(&self, name: &str) -> bool;
    async fn update_symphony(&self, symphony: Symphony) -> bool;
}

pub struct InMemorySymphonyRepository {
    store: SharedStore,
}

impl InMemorySymphonyRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl SymphonyRepository for InMemorySymphonyRepository {
    async fn get_all_symphonies(&self) -> Vec<Symphony> {
        self.store.lock().await.get_all_symphonies()
    }

    async fn get_all_symphonies_with_notes(&self) -> Vec<SymphonyWithNotes> {
        self.store.lock().await.get_all_symphonies_with_notes()
    }
    async fn get_symphony(&self, name: &str) -> Option<Symphony> {
        self.store.lock().await.get_symphony(name)
    }

    async fn get_symphony_with_notes(&self, name: &str) -> Option<SymphonyWithNotes> {
        self.store.lock().await.get_symphony_with_notes(name)
    }

    async fn add_symphony(&self, symphony: Symphony) {
        self.store.lock().await.add_symphony(symphony)
    }

    async fn start_symphony(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        let Some(mut symphony) = store.get_symphony(name) else {
            return false;
        };

        symphony.state = SymphonyState::Running;
        store.update_symphony(symphony).is_ok()
    }

    async fn stop_symphony(&self, name: &str) -> bool {
        let mut store = self.store.lock().await;
        let Some(mut symphony) = store.get_symphony(name) else {
            return false;
        };

        symphony.state = SymphonyState::Terminating;
        store.update_symphony(symphony).is_ok()
    }

    async fn remove_symphony(&self, name: &str) -> bool {
        self.store.lock().await.remove_symphony(name);
        true
    }

    async fn update_symphony(&self, symphony: Symphony) -> bool {
        self.store.lock().await.update_symphony(symphony).is_ok()
    }
}
