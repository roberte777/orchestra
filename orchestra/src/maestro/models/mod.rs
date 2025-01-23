use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use watcher::Watchable;

use super::dto::SymphonyWithNotes;

#[derive(Clone, Serialize, Deserialize)]
pub enum PrincipalState {
    Ready,
    NotReady,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum NoteState {
    Pending,
    Running,
    // if the process exits by iteslf with a zero exit code
    Completed,
    //if a process exits by itself with a non-zero exit code
    Crashed,
    // if a user terminates a process
    Terminated,
    // in the process of shutting down
    Terminating,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Note {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
    pub symphony: String,
    pub state: NoteState,
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

// the states a symphony can be in.
// to it. It moves to terminating while waiting for all notes to be terminated
// by principals. No new notes can be added
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymphonyState {
    // A symphony is running as soon as it is in the database and notes can be
    // added to it.
    Running,
    // A symphony moves to the terminating step when the user stops the symphony
    // and the principal is closing all notes. New notes cannot be added to a
    // symphony that is terminating
    Terminating,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Symphony {
    pub name: String,
    pub notes: Vec<String>,
    pub state: SymphonyState,
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

    pub fn state(&self) -> &SymphonyState {
        &self.state
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Principal {
    pub host: String,
    pub capabilities: Vec<String>,
    pub state: PrincipalState,
    pub last_updated: u64,
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

pub type NoteKey = (String, String);

// Define an in-memory store struct with HashMaps to store each entity type
#[derive(Default)]
pub(crate) struct InMemoryStore {
    notes: HashMap<NoteKey, Note>,
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
        self.notes
            .insert((note.symphony.clone(), note.name.clone()), note);
    }

    pub fn add_symphony(&mut self, symphony: Symphony) {
        self.symphonies.insert(symphony.name.clone(), symphony);
    }

    pub fn add_principal(&mut self, principal: Principal) {
        self.principals.insert(principal.host.clone(), principal);
    }

    // Methods to retrieve data
    pub fn get_note(&self, symphony: &str, name: &str) -> Option<Note> {
        self.notes.get(&(symphony.into(), name.into())).cloned()
    }

    pub fn get_symphony(&self, name: &str) -> Option<Symphony> {
        self.symphonies.get(name).cloned()
    }

    pub fn get_symphony_with_notes(&self, name: &str) -> Option<SymphonyWithNotes> {
        let symphony = self.symphonies.get(name).cloned()?;
        let mut notes = Vec::new();
        for note in symphony.notes() {
            let note = self.get_note(&symphony.name, &note).unwrap();
            notes.push(note);
        }
        let final_symphony = SymphonyWithNotes {
            name: symphony.name.clone(),
            notes,
        };
        Some(final_symphony)
    }

    pub fn get_principal(&self, host: &str) -> Option<Principal> {
        self.principals.get(host).cloned()
    }

    pub fn get_all_notes(&self) -> Vec<Note> {
        self.notes.iter().map(|n| n.1.clone()).collect()
    }

    pub fn get_all_symphonies(&self) -> Vec<Symphony> {
        self.symphonies.iter().map(|n| n.1.clone()).collect()
    }

    pub fn get_all_symphonies_with_notes(&self) -> Vec<SymphonyWithNotes> {
        self.symphonies
            .iter()
            .map(|n| self.get_symphony_with_notes(&n.1.name).unwrap())
            .collect()
    }

    pub fn get_all_principals(&self) -> Vec<Principal> {
        self.principals.iter().map(|n| n.1.clone()).collect()
    }

    // Methods to update data (example for notes)
    pub fn _update_note_state(&mut self, symphony: &str, name: &str, new_state: NoteState) {
        if let Some(note) = self.notes.get_mut(&(symphony.into(), name.into())) {
            note.state = new_state;
        }
    }

    // Update or replace a Note completely
    pub fn update_note(&mut self, updated_note: Note) -> Result<(), String> {
        let key = &(updated_note.symphony.clone(), updated_note.name.clone());
        if self.notes.contains_key(key) {
            self.notes.insert(key.clone(), updated_note);
            Ok(())
        } else {
            Err(format!("Note '{}:{}' not found", key.0, key.1))
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
    pub fn remove_note(&mut self, symphony: &str, name: &str) {
        self.notes.remove(&(symphony.into(), name.into()));
    }

    pub fn remove_symphony(&mut self, name: &str) {
        self.symphonies.remove(name);
    }

    pub fn _remove_principal(&mut self, host: &str) {
        self.principals.remove(host);
    }
}

// To ensure safe concurrent access, wrap the InMemoryStore in an Arc<Mutex<>>
pub(crate) type SharedStore = Arc<Mutex<InMemoryStore>>;

pub trait SharedStoreExt {
    fn new_shared() -> Self;
}

impl SharedStoreExt for SharedStore {
    fn new_shared() -> Self {
        Arc::new(Mutex::new(InMemoryStore::new()))
    }
}
