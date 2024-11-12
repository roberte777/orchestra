use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use watcher::Watchable;

#[derive(Clone)]
pub enum PrincipalState {
    Ready,
    NotReady,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum NoteState {
    Pending,
    Running,
    Completed,
    Terminated,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DesiredState {
    Run,
    Stop,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Note {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: String,
    pub symphony: String,
    pub state: NoteState,
    pub desired_state: DesiredState,
}

impl Watchable for Note {
    fn get_field_value(&self, field_name: &str) -> Option<String> {
        match field_name {
            "name" => Some(self.name.clone()),
            "description" => Some(self.description.clone()),
            "host" => Some(self.host.clone()),
            "command" => Some(self.command.clone()),
            "symphony" => Some(self.symphony.clone()),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Symphony {
    name: String,
    notes: Vec<String>,
    desired_state: DesiredState,
}

impl Symphony {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns a list of note names that are associated with this symphony.
    /// Use the repository to additionally search for the associated notes
    pub fn notes(&self) -> Vec<String> {
        self.notes.clone()
    }

    pub fn desired_state(&self) -> DesiredState {
        self.desired_state.clone()
    }
}

impl Watchable for Symphony {
    fn get_field_value(&self, field_name: &str) -> Option<String> {
        match field_name {
            "name" => Some(self.name.clone()),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Principal {
    host: String,
    capabilities: Vec<String>,
    state: PrincipalState,
    last_updated: u64,
}

impl Principal {
    pub fn host(&self) -> String {
        self.host.clone()
    }

    pub fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    pub fn state(&self) -> PrincipalState {
        self.state.clone()
    }

    pub fn last_updated(&self) -> u64 {
        self.last_updated
    }
}

impl Watchable for Principal {
    fn get_field_value(&self, field_name: &str) -> Option<String> {
        match field_name {
            "host" => Some(self.host.clone()),
            _ => None,
        }
    }
}
// Define an in-memory store struct with HashMaps to store each entity type
#[derive(Default)]
pub struct InMemoryStore {
    notes: HashMap<String, Note>,
    symphonies: HashMap<String, Symphony>,
    principals: HashMap<String, Principal>,
}

impl InMemoryStore {
    // Initialize a new store
    pub fn new() -> Self {
        InMemoryStore {
            notes: HashMap::new(),
            symphonies: HashMap::new(),
            principals: HashMap::new(),
        }
    }

    // Methods to add data to the store
    pub fn add_note(&mut self, note: Note) {
        self.notes.insert(note.name.clone(), note);
    }

    pub fn add_symphony(&mut self, symphony: Symphony) {
        self.symphonies.insert(symphony.name.clone(), symphony);
    }

    pub fn add_principal(&mut self, principal: Principal) {
        self.principals.insert(principal.host.clone(), principal);
    }

    // Methods to retrieve data
    pub fn get_note(&self, name: &str) -> Option<Note> {
        self.notes.get(name).cloned()
    }

    pub fn get_symphony(&self, name: &str) -> Option<Symphony> {
        self.symphonies.get(name).cloned()
    }

    pub fn get_principal(&self, host: &str) -> Option<Principal> {
        self.principals.get(host).cloned()
    }

    pub fn get_all_notes(&self) -> Vec<Note> {
        self.notes.iter().map(|n| n.1.clone()).collect()
    }

    // Methods to update data (example for notes)
    pub fn update_note_state(&mut self, name: &str, new_state: NoteState) {
        if let Some(note) = self.notes.get_mut(name) {
            note.state = new_state;
        }
    }

    // Update or replace a Note completely
    pub fn update_note(&mut self, updated_note: Note) -> Result<(), String> {
        let name = &updated_note.name;
        if self.notes.contains_key(name) {
            self.notes.insert(name.clone(), updated_note);
            Ok(())
        } else {
            Err(format!("Note '{}' not found", name))
        }
    }

    // Update or replace a Symphony completely
    pub fn update_symphony(&mut self, updated_symphony: Symphony) -> Result<(), String> {
        let name = &updated_symphony.name;
        if self.symphonies.contains_key(name) {
            self.symphonies.insert(name.clone(), updated_symphony);
            Ok(())
        } else {
            Err(format!("Symphony '{}' not found", name))
        }
    }

    // Update or replace a Principal completely
    pub fn update_principal(&mut self, updated_principal: Principal) -> Result<(), String> {
        let host = &updated_principal.host;
        if self.principals.contains_key(host) {
            self.principals.insert(host.clone(), updated_principal);
            Ok(())
        } else {
            Err(format!("Principal '{}' not found", host))
        }
    }

    // Methods to remove data if needed
    pub fn remove_note(&mut self, name: &str) {
        self.notes.remove(name);
    }

    pub fn remove_symphony(&mut self, name: &str) {
        self.symphonies.remove(name);
    }

    pub fn remove_principal(&mut self, host: &str) {
        self.principals.remove(host);
    }
}

// To ensure safe concurrent access, wrap the InMemoryStore in an Arc<Mutex<>>
pub type SharedStore = Arc<Mutex<InMemoryStore>>;

pub trait SharedStoreExt {
    fn new_shared() -> Self;
}

impl SharedStoreExt for SharedStore {
    fn new_shared() -> Self {
        Arc::new(Mutex::new(InMemoryStore::new()))
    }
}
