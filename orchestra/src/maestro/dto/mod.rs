use serde::{Deserialize, Serialize};
use watcher::Watchable;

use super::models::{Note, SymphonyState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymphonyWithNotes {
    pub name: String,
    pub notes: Vec<Note>,
    pub state: SymphonyState,
}

impl SymphonyWithNotes {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns a list of note names that are associated with this symphony.
    /// Use the repository to additionally search for the associated notes
    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }

    pub fn state(&self) -> &SymphonyState {
        &self.state
    }
}

impl Watchable for SymphonyWithNotes {
    fn get_field_value(&self, field_name: &str) -> Option<String> {
        match field_name {
            "name" => Some(self.name.clone()),
            _ => None,
        }
    }
}
