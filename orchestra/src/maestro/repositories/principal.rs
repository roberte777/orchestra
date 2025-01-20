use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::maestro::models::{Principal, SharedStore};

#[async_trait]
pub trait PrincipalRepository: Send + Sync {
    async fn get_all_principals(&self) -> Vec<Principal>;
    async fn upsert_principal(&self, principal: Principal);
}

pub struct InMemoryPrincipalRepository {
    pub store: SharedStore,
}

impl InMemoryPrincipalRepository {
    pub fn new(store: SharedStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl PrincipalRepository for InMemoryPrincipalRepository {
    async fn get_all_principals(&self) -> Vec<Principal> {
        let store = self.store.lock().await;
        store.get_all_principals()
    }

    async fn upsert_principal(&self, principal: Principal) {
        let mut store = self.store.lock().await;
        // If the principal already exists, update; otherwise, insert:
        let host = principal.host.clone();
        if store.get_principal(&host).is_some() {
            let _ = store.update_principal(principal);
        } else {
            store.add_principal(principal);
        }
    }
}
