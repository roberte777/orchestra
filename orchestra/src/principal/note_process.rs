use std::{
    collections::{HashMap, HashSet},
    ops::DerefMut,
    process::Command,
    sync::Arc,
};

use shared_child::SharedChild;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, instrument, warn, Instrument};

use crate::maestro::models::{DesiredState, Note, NoteState, RestartPolicy};

use super::AppState;

#[derive(Debug)]
pub struct ManagedNote {
    pub note: Note,
    pub child: Option<Arc<SharedChild>>,
}

#[instrument]
pub async fn start_note(
    notes: &mut HashMap<String, ManagedNote>,
    mut note: Note,
    exit_tx: UnboundedSender<(String, i32)>,
) {
    if !matches!(note.desired_state, DesiredState::Run) {
        warn!("Tried to start a process that does not have a desired state of Run");
        return;
    }
    if let Some(proc_entry) = notes.get(&note.name) {
        if matches!(proc_entry.note.state, NoteState::Running) {
            warn!("Process is already running. Ignoring start.");
            return;
        }

        // if note is not running, there should be no child
        if proc_entry.child.is_some() {
            error!("ManagedNote has a child even though it is not running.");
            return;
        }
    }

    debug!("Starting process");

    let mut command = Command::new(note.command.clone());
    command.args(note.args.clone()).envs(note.env.clone());

    match SharedChild::spawn(&mut command) {
        Ok(child) => {
            note.state = NoteState::Running;
            let child = Arc::new(child);
            let exit_name = note.name.clone();
            {
                let child = Arc::clone(&child);
                let managed_note = ManagedNote {
                    note,
                    child: Some(child),
                };
                notes
                    .entry(managed_note.note.name.clone())
                    .insert_entry(managed_note);
            }
            let child = Arc::clone(&child);
            _ = std::thread::spawn(move || {
                let status = child.wait();
                let code = status
                    .map(|exit_status| exit_status.code().unwrap_or(-1))
                    .unwrap_or(-1);
                if let Err(e) = exit_tx.send((exit_name, code)) {
                    error!("Failed to send process exit notification: {}", e);
                }
            })
            .in_current_span();
        }
        Err(e) => {
            warn!(command = ?command, "Failed to spawn command: {}", e);
            note.state = NoteState::Terminated;
            let managed_note = ManagedNote { note, child: None };
            notes
                .entry(managed_note.note.name.clone())
                .insert_entry(managed_note);
        }
    }
}

#[instrument]
pub async fn stop_note(notes: &mut HashMap<String, ManagedNote>, name: &str) {
    match notes.get_mut(name) {
        Some(managed_proc) => {
            if let Some(child) = &managed_proc.child {
                debug!("Killing process: {}", name);
                // Mark it as Killed by command
                managed_proc.note.state = NoteState::Terminated;
                let _ = child.kill();
            } else {
                warn!("Process {} has no active child to kill.", name);
            }
        }
        None => {
            warn!("Cannot find note");
        }
    }
}

pub async fn synchronize_state(state: &mut AppState, override_notes: Vec<Note>) {
    let mut notes = state.notes.lock().await;

    // Create a set of names from the override list for quick lookups
    let override_names: HashSet<String> = override_notes
        .iter()
        .map(|note| note.name.clone())
        .collect();

    // Remove notes from the current state that are not in the override list
    notes.retain(|name, managed_note| {
        if !override_names.contains(name) {
            info!("Removing note '{}' not present in the override list.", name);
            if let Some(child) = &managed_note.child {
                info!("Stopping child process for note '{}'.", name);
                let _ = child.kill();
            }
            false // Remove this entry
        } else {
            true // Keep this entry
        }
    });

    // Add or update notes from the override list
    for override_note in override_notes {
        match notes.get_mut(&override_note.name) {
            Some(managed_note) => {
                // Update the note if it exists
                info!("Updating existing note '{}'.", override_note.name);
                managed_note.note = override_note.clone();

                // Handle running state if necessary
                if !matches!(managed_note.note.desired_state, DesiredState::Run) {
                    if let Some(child) = &managed_note.child {
                        info!(
                            "Stopping child process for note '{}' due to updated desired state.",
                            override_note.name
                        );
                        let _ = child.kill();
                        managed_note.child = None;
                    }
                }
            }
            None => {
                // Add the new note
                info!("Adding new note '{}'.", override_note.name);
                let managed_note = ManagedNote {
                    note: override_note.clone(),
                    child: None,
                };
                notes.insert(override_note.name.clone(), managed_note);
            }
        }
    }
}

/// Receives a message when a process exits and updates its state.
/// If needed, it restarts the process depending on its restart policy.
pub async fn handle_process_exits(
    state: &mut AppState,
    tx: UnboundedSender<(String, i32)>,
    mut rx: UnboundedReceiver<(String, i32)>,
) {
    while let Some((proc_name, exit_code)) = rx.recv().await {
        info!("Process '{}' exited.", proc_name);
        let mut processes = state.notes.lock().await;

        let mut restart_process = false;

        if let Some(proc_entry) = processes.get_mut(&proc_name) {
            // If we didn't mark it as Killed earlier, then it is Exited
            if matches!(proc_entry.note.state, NoteState::Terminated) {
                // don't attempt to restart if terminated by user.
                return;
            }

            if exit_code == 0 {
                proc_entry.note.state = NoteState::Completed;
            } else {
                proc_entry.note.state = NoteState::Crashed;
            }

            // Drop the old child (so we can start a fresh one if we need)
            proc_entry.child = None;

            match proc_entry.note.restart_policy {
                RestartPolicy::Always => {
                    // We'll do a quick re-start
                    info!("Restarting process {} due to 'always' policy.", proc_name);
                    restart_process = true;
                }
                RestartPolicy::OnFailure => {
                    // A simple example: if exit_code != 0, restart
                    if exit_code != 0 {
                        info!(
                            "Restarting process {} due to 'on-failure' policy.",
                            proc_name
                        );
                        restart_process = true;
                    }
                }
                _ => {
                    // "never" or unrecognized
                }
            }
        }
        if restart_process {
            let note = {
                let processes = state.notes.lock().await;
                if let Some(proc_entry) = processes.get(&proc_name) {
                    proc_entry.note.clone()
                } else {
                    continue;
                }
            };

            start_note(state.notes.lock().await.deref_mut(), note, tx.clone()).await;
        }
    }

    info!("Exiting 'handle_process_exits' thread");
}
