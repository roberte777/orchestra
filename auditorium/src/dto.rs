use std::collections::HashMap;

use orchestra::maestro::models::{Note, RestartPolicy};
use serde::{Deserialize, Serialize};

/// Example minimal Maestro symphony shape
/// that we get from GET /api/v1/symphonies in Maestro
#[derive(Clone, Debug, Deserialize)]
pub struct MaestroSymphony {
    pub name: String,
    pub notes: Vec<Note>,
}

// data structure sent by the user to create a symphony
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSymphony {
    pub name: String,
    pub notes: Vec<CreateSymphonyNote>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSymphonyNote {
    pub name: String,
    pub description: String,
    pub host: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
}
