use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    maestro::models::{NoteState, RestartPolicy},
    principal::{note_process::start_note, AppState},
};

/// This struct owns the background task that handles process exits.
/// When you call `.start()`, it spawns the background task.
/// When you call `.stop()`, it cancels the task and waits for it to exit.
///
/// Internally, it uses a `CancellationToken` to interrupt the loop, then kills
/// any remaining processes before returning.
#[derive(Default)]
pub struct ProcessExitsActor {
    join_handle: Option<JoinHandle<()>>,
    cancel: CancellationToken,
}

impl ProcessExitsActor {
    /// Create a new actor. Does not start the background task yet.
    pub fn new() -> Self {
        ProcessExitsActor {
            join_handle: None,
            cancel: CancellationToken::new(),
        }
    }

    /// Start the background task that listens on `rx` for `(proc_name, exit_code)` messages,
    /// and updates the internal `AppState` accordingly.
    ///
    /// If the background task is already running, this does nothing.
    pub fn start(
        &mut self,
        state: AppState,
        exit_tx: UnboundedSender<(String, i32)>,
        mut exit_rx: UnboundedReceiver<(String, i32)>,
    ) {
        // If we already have a running task, do nothing:
        if self.join_handle.is_some() {
            warn!("ProcessExitsActor is already running. Ignoring .start().");
            return;
        }

        let cancel = self.cancel.child_token();

        // Spawn the background task:
        let jh = tokio::spawn(async move {
            loop {
                // Either read from exit_rx, or see if we were cancelled
                tokio::select! {
                    maybe_msg = exit_rx.recv() => {
                        // If channel was closed, or we got None, break out
                        let (proc_name, exit_code) = match maybe_msg {
                            Some(x) => x,
                            None => {
                                // All senders dropped => no more events
                                info!("All senders dropped. Exiting process_exits task.");
                                break;
                            }
                        };

                        info!("Process '{}' exited with code {}.", proc_name, exit_code);

                        // Lock the notes for processing
                        let mut processes = state.notes.lock().await;
                        let mut restart_process = false;

                        // If this process still exists in the map, handle exit logic
                        if let Some(proc_entry) = processes.get_mut(&proc_name) {
                            // If we didn't mark it as Killed earlier, then it is Exited
                            if matches!(proc_entry.note.state, NoteState::Terminated) {
                                info!(note = proc_entry.note.name, "process terminated");
                                // if it's manually terminated, don't restart
                                continue;
                            }

                            // Set correct status
                            if exit_code == 0 {
                                proc_entry.note.state = NoteState::Completed;
                            } else {
                                proc_entry.note.state = NoteState::Crashed;
                            }

                            // Drop the old child (so we can start a fresh one if needed)
                            proc_entry.child = None;

                            // Evaluate restart policy
                            match proc_entry.note.restart_policy {
                                RestartPolicy::Always => {
                                    info!("Restarting process {} due to 'always' policy.", proc_name);
                                    restart_process = true;
                                }
                                RestartPolicy::OnFailure => {
                                    if exit_code != 0 {
                                        info!("Restarting process {} due to 'on-failure' policy.", proc_name);
                                        restart_process = true;
                                    }
                                }
                                _ => {
                                    // "never" or unrecognized => do nothing
                                }
                            }
                        }

                        // If we need to restart, we re-acquire the note and start it
                        if restart_process {
                            let note = {
                                if let Some(proc_entry) = processes.get(&proc_name) {
                                    proc_entry.note.clone()
                                } else {
                                    // note was removed from map => do not restart
                                    continue;
                                }
                            };

                            // Re-start note
                            start_note(
                                &mut processes,
                                note,
                                exit_tx.clone()
                            ).await;
                        }
                    }

                    // If we’re cancelled, break out of the loop
                    _ = cancel.cancelled() => {
                        info!("ProcessExitsActor received cancellation. Preparing to exit.");
                        break;
                    }
                }
            }

            info!("Exiting 'handle_process_exits' thread");
        });

        self.join_handle = Some(jh);
    }

    /// Cancels the background task, waits for it to finish, and returns.
    ///
    /// If the background task is not running, this does nothing.
    pub async fn stop(&mut self) {
        if let Some(jh) = self.join_handle.take() {
            // Cancel the loop
            self.cancel.cancel();

            // Wait for the background task to finish
            let _ = jh.await;
        } else {
            warn!("ProcessExitsActor is not running. Ignoring .stop().");
        }
    }
}
