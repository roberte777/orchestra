use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use watcher::Watchable;

// ---------------------
// Entities
// ---------------------

#[derive(Clone, Serialize, Deserialize)]
pub enum PrincipalState {
    Ready,
    NotReady,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum NoteState {
    Pending,
    Running,
    Completed,
    Crashed,
    Terminated,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DesiredState {
    Run,
    Stop,
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
    pub symphony: String, // parent symphony name
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

#[derive(Clone, Debug)]
pub struct Symphony {
    pub name: String,
    pub notes: Vec<String>, // a list of note names
    pub desired_state: DesiredState,
}

impl Symphony {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn notes(&self) -> &[String] {
        &self.notes
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Principal {
    pub host: String,
    pub capabilities: Vec<String>,
    pub state: PrincipalState,
    pub last_updated: u64,
}

impl Principal {
    pub fn host(&self) -> &str {
        &self.host
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

// ---------------------
// InMemoryStore (master data store)
// ---------------------

#[derive(Default)]
pub struct InMemoryStore {
    // You could make these private if you prefer
    pub notes: HashMap<String, Note>,
    pub symphonies: HashMap<String, Symphony>,
    pub principals: HashMap<String, Principal>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            notes: HashMap::new(),
            symphonies: HashMap::new(),
            principals: HashMap::new(),
        }
    }

    // =============== NOTE OPERATIONS ===============
    pub fn add_note(&mut self, note: Note) {
        self.notes.insert(note.name.clone(), note);
    }
    pub fn get_note(&self, name: &str) -> Option<Note> {
        self.notes.get(name).cloned()
    }
    pub fn update_note(&mut self, note: Note) -> Result<(), String> {
        let name = &note.name;
        if self.notes.contains_key(name) {
            self.notes.insert(name.clone(), note);
            Ok(())
        } else {
            Err(format!("Note '{}' not found", name))
        }
    }
    pub fn remove_note(&mut self, name: &str) -> Option<Note> {
        self.notes.remove(name)
    }
    pub fn get_all_notes(&self) -> Vec<Note> {
        self.notes.values().cloned().collect()
    }

    // =============== SYMPHONY OPERATIONS ===============
    pub fn add_symphony(&mut self, symphony: Symphony) {
        self.symphonies.insert(symphony.name.clone(), symphony);
    }
    pub fn get_symphony(&self, name: &str) -> Option<Symphony> {
        self.symphonies.get(name).cloned()
    }
    pub fn update_symphony(&mut self, symphony: Symphony) -> Result<(), String> {
        let name = &symphony.name;
        if self.symphonies.contains_key(name) {
            self.symphonies.insert(name.clone(), symphony);
            Ok(())
        } else {
            Err(format!("Symphony '{}' not found", name))
        }
    }
    pub fn remove_symphony(&mut self, name: &str) -> Option<Symphony> {
        self.symphonies.remove(name)
    }
    pub fn get_all_symphonies(&self) -> Vec<Symphony> {
        self.symphonies.values().cloned().collect()
    }

    // =============== PRINCIPAL OPERATIONS ===============
    pub fn add_principal(&mut self, principal: Principal) {
        self.principals.insert(principal.host.clone(), principal);
    }
    pub fn get_principal(&self, host: &str) -> Option<Principal> {
        self.principals.get(host).cloned()
    }
    pub fn update_principal(&mut self, updated: Principal) -> Result<(), String> {
        let host = &updated.host;
        if self.principals.contains_key(host) {
            self.principals.insert(host.clone(), updated);
            Ok(())
        } else {
            Err(format!("Principal '{}' not found", host))
        }
    }
    pub fn remove_principal(&mut self, host: &str) -> Option<Principal> {
        self.principals.remove(host)
    }
    pub fn get_all_principals(&self) -> Vec<Principal> {
        self.principals.values().cloned().collect()
    }

    // ----------------------------------------------------
    // ========== TRANSACTION-LIKE CONVENIENCE METHODS =====
    // ----------------------------------------------------

    /// Create a new Symphony *and* all of its notes in a single “transaction.”
    /// - If the Symphony name already exists, returns an error.
    /// - If *any* of the note names already exist, returns an error.
    ///
    /// On success, both the Symphony and its Notes are inserted at once.
    pub fn add_symphony_with_notes(
        &mut self,
        symphony: Symphony,
        notes: Vec<Note>,
    ) -> Result<(), String> {
        // Check for existing symphony:
        if self.symphonies.contains_key(&symphony.name) {
            return Err(format!("Symphony '{}' already exists", symphony.name));
        }
        // Check for existing note names:
        for note in &notes {
            if self.notes.contains_key(&note.name) {
                return Err(format!("Note '{}' already exists", note.name));
            }
        }

        // Insert the new Symphony
        self.add_symphony(symphony.clone());

        // Insert each Note
        for note in notes {
            // We could also check that note.symphony == symphony.name
            // but that might be optional
            self.add_note(note);
        }

        Ok(())
    }

    /// Remove a Symphony and all of its associated notes in one pass.
    /// If `remove_only_if_stopped` is `true`, this can check `desired_state`
    /// or your business logic, but that’s up to you.
    pub fn remove_symphony_and_notes(
        &mut self,
        symphony_name: &str,
        remove_only_if_stopped: bool,
    ) -> Result<(), String> {
        let symphony = match self.symphonies.get(symphony_name) {
            Some(s) => s.clone(),
            None => return Err(format!("Symphony '{}' not found", symphony_name)),
        };

        // If you want to check the symphony is "Stop" before removing:
        if remove_only_if_stopped && !matches!(symphony.desired_state, DesiredState::Stop) {
            return Err(format!(
                "Symphony '{}' is not in the STOP state, cannot remove",
                symphony_name
            ));
        }

        // Remove the symphony from the map
        self.remove_symphony(symphony_name);

        // Remove all notes that belong to this symphony
        for note_name in &symphony.notes {
            self.remove_note(note_name);
        }

        Ok(())
    }

    /// Update a Symphony and its notes in a single pass. (Pseudo-transaction.)
    /// For example, you might replace the entire symphony structure *and*
    /// upsert or remove any changed notes to match that new structure.
    pub fn update_symphony_with_notes(
        &mut self,
        symphony: Symphony,
        notes: Vec<Note>,
    ) -> Result<(), String> {
        // 1. The symphony must already exist:
        if !self.symphonies.contains_key(&symphony.name) {
            return Err(format!("Symphony '{}' not found", symphony.name));
        }

        // 2. Perform the update on the symphony:
        self.update_symphony(symphony.clone())?;

        // 3. Let’s say you want to remove existing notes that are no longer in `notes`.
        //    First we gather the new note names:
        let new_note_names: Vec<String> = notes.iter().map(|n| n.name.clone()).collect();

        // 4. Remove any old notes that are not in the new note list
        //    (We know they belong to this symphony by checking `symphony.notes` or
        //     or you can rely on each note’s `symphony` field.)
        let mut to_remove = Vec::new();
        if let Some(existing_symphony) = self.symphonies.get(&symphony.name) {
            for old_note_name in &existing_symphony.notes {
                if !new_note_names.contains(old_note_name) {
                    to_remove.push(old_note_name.clone());
                }
            }
        }
        for name in to_remove {
            self.remove_note(&name);
        }

        // 5. Update or insert the new notes
        for note in notes {
            // The note must be linked to the same symphony
            if note.symphony != symphony.name {
                return Err(format!(
                    "Note '{}' does not reference symphony '{}'",
                    note.name, symphony.name
                ));
            }

            // If the note already exists, update it. If not, add it.
            // This example calls “update_note” but if that fails, we add it:
            match self.update_note(note.clone()) {
                Ok(_) => { /* updated existing note */ }
                Err(_) => {
                    // The note didn't exist, so add it:
                    self.add_note(note);
                }
            }
        }

        Ok(())
    }
}

// For concurrency safety
pub type SharedStore = Arc<Mutex<InMemoryStore>>;

pub trait SharedStoreExt {
    fn new_shared() -> Self;
}

impl SharedStoreExt for SharedStore {
    fn new_shared() -> Self {
        Arc::new(Mutex::new(InMemoryStore::new()))
    }
}
