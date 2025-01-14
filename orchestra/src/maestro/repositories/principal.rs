use crate::maestro::models::{Principal, SharedStore};
use async_trait::async_trait;

#[async_trait]
pub trait PrincipalRepository: Send + Sync {
    async fn get_all_principals(&self) -> Vec<Principal>;
    async fn upsert_principal(&self, principal: Principal);
}

pub struct InMemoryPrincipalRepository {
    store: SharedStore,
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

        // check if principal exists
        if store.get_principal(&principal.host()).is_some() {
            // if it exists, update
            _ = store.update_principal(principal);
        } else {
            store.add_principal(principal);
        }
    }
}
