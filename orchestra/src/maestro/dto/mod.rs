use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use watcher::Watchable;

use super::models::{Note, NoteState, RestartPolicy, Symphony, SymphonyState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymphonyWithNotes {
    pub name: String,
    pub notes: Vec<Note>,
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
}

impl Watchable for SymphonyWithNotes {
    fn get_field_value(&self, field_name: &str) -> Option<String> {
        match field_name {
            "name" => Some(self.name.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct CreateNote {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
}

impl CreateNote {
    pub fn to_data_obj(&self, symphony_name: &str) -> Note {
        Note {
            name: self.name.clone(),
            description: self.description.clone(),
            host: self.host.clone(),
            env: self.env.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            restart_policy: self.restart_policy.clone(),
            state: NoteState::Pending,
            symphony: symphony_name.to_string(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct CreateSymphony {
    pub name: String,
    pub notes: Vec<CreateNote>,
}

impl CreateSymphony {
    pub fn to_data_obj(&self) -> Symphony {
        let notes = self.notes.iter().map(|n| n.name.clone()).collect();
        Symphony {
            notes,
            name: self.name.clone(),
            state: SymphonyState::Running,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartbeatDto {
    pub name: String,
    pub notes: Vec<HeartbeatNoteDto>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartbeatNoteDto {
    pub name: String,
    pub state: NoteState,
}
