use crate::types::{AppState, ViewMode, ConflictResolution};
use crate::websocket::WebSocketClient;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::{debug, error, info, warn};

/// Handle keyboard events and update application state
pub fn handle_key_event(
    app_state: &mut AppState,
    key_event: KeyEvent,
    ws_client: &WebSocketClient,
) -> Result<()> {
    match key_event.code {
        // Global quit keys
        KeyCode::Char('q') | KeyCode::Esc if matches!(app_state.current_view, ViewMode::Home) => {
            app_state.should_quit = true;
        }
        
        // Global help
        KeyCode::Char('h') | KeyCode::Char('?') => {
            app_state.current_view = ViewMode::Help;
        }

        // Global view navigation
        KeyCode::Char('1') => {
            app_state.current_view = ViewMode::Home;
            app_state.selected_job_index = 0;
        }
        KeyCode::Char('2') => {
            app_state.current_view = ViewMode::Settings;
        }

        // Handle events based on current view
        _ => match app_state.current_view.clone() {
            ViewMode::Home => handle_home_keys(app_state, key_event, ws_client)?,
            ViewMode::JobDetail(job_id) => handle_job_detail_keys(app_state, key_event, ws_client, job_id)?,
            ViewMode::ConflictResolution(job_id) => handle_conflict_keys(app_state, key_event, ws_client, job_id)?,
            ViewMode::Help => handle_help_keys(app_state, key_event)?,
            ViewMode::Settings => handle_settings_keys(app_state, key_event)?,
        },
    }

    Ok(())
}

/// Handle keyboard events in the home view
fn handle_home_keys(
    app_state: &mut AppState,
    key_event: KeyEvent,
    ws_client: &WebSocketClient,
) -> Result<()> {
    match key_event.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app_state.move_selection_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app_state.move_selection_down();
        }

        // Enter job detail view
        KeyCode::Enter => {
            if let Some(job) = app_state.get_selected_job() {
                app_state.current_view = ViewMode::JobDetail(job.id);
                app_state.selected_log_index = 0;
            }
        }

        // Job control actions
        KeyCode::Char('s') => {
            if let Some(job) = app_state.get_selected_job() {
                if job.can_start() {
                    info!("Starting job: {}", job.name);
                    if let Err(e) = ws_client.start_job(&job.id.to_string()) {
                        error!("Failed to start job: {}", e);
                    }
                } else {
                    warn!("Job cannot be started in current state: {}", job.status);
                }
            }
        }

        KeyCode::Char('p') => {
            if let Some(job) = app_state.get_selected_job() {
                if job.can_pause() {
                    info!("Pausing job: {}", job.name);
                    if let Err(e) = ws_client.pause_job(&job.id.to_string()) {
                        error!("Failed to pause job: {}", e);
                    }
                } else {
                    warn!("Job cannot be paused in current state: {}", job.status);
                }
            }
        }

        KeyCode::Char('t') => {
            if let Some(job) = app_state.get_selected_job() {
                if job.can_stop() {
                    info!("Stopping job: {}", job.name);
                    if let Err(e) = ws_client.stop_job(&job.id.to_string()) {
                        error!("Failed to stop job: {}", e);
                    }
                } else {
                    warn!("Job cannot be stopped in current state: {}", job.status);
                }
            }
        }

        KeyCode::Char('r') => {
            if let Some(job) = app_state.get_selected_job() {
                info!("Retrying job: {}", job.name);
                if let Err(e) = ws_client.retry_job(&job.id.to_string()) {
                    error!("Failed to retry job: {}", e);
                }
            }
        }

        // View conflicts
        KeyCode::Char('c') => {
            if let Some(job) = app_state.get_selected_job() {
                if !job.conflicts.is_empty() {
                    app_state.current_view = ViewMode::ConflictResolution(job.id);
                    app_state.selected_conflict_index = 0;
                } else {
                    debug!("No conflicts to resolve for job: {}", job.name);
                }
            }
        }

        // Page navigation
        KeyCode::PageUp => {
            for _ in 0..5 {
                app_state.move_selection_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..5 {
                app_state.move_selection_down();
            }
        }

        // Home key - go to first job
        KeyCode::Home => {
            app_state.selected_job_index = 0;
        }

        // End key - go to last job
        KeyCode::End => {
            let job_count = app_state.jobs.len();
            if job_count > 0 {
                app_state.selected_job_index = job_count - 1;
            }
        }

        _ => {}
    }

    Ok(())
}

/// Handle keyboard events in the job detail view
fn handle_job_detail_keys(
    app_state: &mut AppState,
    key_event: KeyEvent,
    ws_client: &WebSocketClient,
    job_id: uuid::Uuid,
) -> Result<()> {
    match key_event.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app_state.move_selection_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app_state.move_selection_down();
        }

        // Go back to home
        KeyCode::Esc => {
            app_state.current_view = ViewMode::Home;
        }

        // Job control actions (same as home view)
        KeyCode::Char('s') => {
            if let Some(job) = app_state.jobs.get(&job_id) {
                if job.can_start() {
                    info!("Starting job: {}", job.name);
                    if let Err(e) = ws_client.start_job(&job_id.to_string()) {
                        error!("Failed to start job: {}", e);
                    }
                }
            }
        }

        KeyCode::Char('p') => {
            if let Some(job) = app_state.jobs.get(&job_id) {
                if job.can_pause() {
                    info!("Pausing job: {}", job.name);
                    if let Err(e) = ws_client.pause_job(&job_id.to_string()) {
                        error!("Failed to pause job: {}", e);
                    }
                }
            }
        }

        KeyCode::Char('t') => {
            if let Some(job) = app_state.jobs.get(&job_id) {
                if job.can_stop() {
                    info!("Stopping job: {}", job.name);
                    if let Err(e) = ws_client.stop_job(&job_id.to_string()) {
                        error!("Failed to stop job: {}", e);
                    }
                }
            }
        }

        KeyCode::Char('r') => {
            info!("Retrying job: {}", job_id);
            if let Err(e) = ws_client.retry_job(&job_id.to_string()) {
                error!("Failed to retry job: {}", e);
            }
        }

        // View conflicts
        KeyCode::Char('c') => {
            if let Some(job) = app_state.jobs.get(&job_id) {
                if !job.conflicts.is_empty() {
                    app_state.current_view = ViewMode::ConflictResolution(job_id);
                    app_state.selected_conflict_index = 0;
                }
            }
        }

        // Page navigation for logs
        KeyCode::PageUp => {
            for _ in 0..5 {
                app_state.move_selection_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..5 {
                app_state.move_selection_down();
            }
        }

        _ => {}
    }

    Ok(())
}

/// Handle keyboard events in the conflict resolution view
fn handle_conflict_keys(
    app_state: &mut AppState,
    key_event: KeyEvent,
    ws_client: &WebSocketClient,
    job_id: uuid::Uuid,
) -> Result<()> {
    match key_event.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app_state.move_selection_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app_state.move_selection_down();
        }

        // Go back to job detail
        KeyCode::Esc => {
            app_state.current_view = ViewMode::JobDetail(job_id);
        }

        // Conflict resolution options
        KeyCode::Char('1') => {
            resolve_selected_conflict(app_state, ws_client, job_id, ConflictResolution::KeepSource)?;
        }
        KeyCode::Char('2') => {
            resolve_selected_conflict(app_state, ws_client, job_id, ConflictResolution::KeepTarget)?;
        }
        KeyCode::Char('3') => {
            resolve_selected_conflict(app_state, ws_client, job_id, ConflictResolution::Merge)?;
        }
        KeyCode::Char('4') => {
            resolve_selected_conflict(app_state, ws_client, job_id, ConflictResolution::Skip)?;
        }

        // Apply resolution with Enter
        KeyCode::Enter => {
            // Show a sub-menu or default to keep source
            resolve_selected_conflict(app_state, ws_client, job_id, ConflictResolution::KeepSource)?;
        }

        // Page navigation
        KeyCode::PageUp => {
            for _ in 0..5 {
                app_state.move_selection_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..5 {
                app_state.move_selection_down();
            }
        }

        _ => {}
    }

    Ok(())
}

/// Handle keyboard events in the help view
fn handle_help_keys(app_state: &mut AppState, _key_event: KeyEvent) -> Result<()> {
    // Any key returns to the previous view (assume Home for now)
    app_state.current_view = ViewMode::Home;
    Ok(())
}

/// Handle keyboard events in the settings view
fn handle_settings_keys(app_state: &mut AppState, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Esc => {
            app_state.current_view = ViewMode::Home;
        }
        _ => {}
    }

    Ok(())
}

/// Resolve the currently selected conflict
fn resolve_selected_conflict(
    app_state: &mut AppState,
    ws_client: &WebSocketClient,
    job_id: uuid::Uuid,
    resolution: ConflictResolution,
) -> Result<()> {
    if let Some(job) = app_state.jobs.get_mut(&job_id) {
        if let Some(conflict) = job.conflicts.get_mut(app_state.selected_conflict_index) {
            if conflict.resolution.is_none() {
                info!(
                    "Resolving conflict {} for job {} with resolution: {}",
                    conflict.id, job_id, resolution
                );

                // Update local state immediately for UI feedback
                conflict.resolution = Some(resolution.clone());

                // Send resolution to backend
                let resolution_str = match resolution {
                    ConflictResolution::KeepSource => "keep_source",
                    ConflictResolution::KeepTarget => "keep_target",
                    ConflictResolution::Merge => "merge",
                    ConflictResolution::Skip => "skip",
                };

                if let Err(e) = ws_client.resolve_conflict(
                    &job_id.to_string(),
                    &conflict.id.to_string(),
                    resolution_str,
                ) {
                    error!("Failed to send conflict resolution: {}", e);
                    // Revert local state on error
                    conflict.resolution = None;
                }
            } else {
                debug!("Conflict already resolved");
            }
        }
    }

    Ok(())
}

/// Check if the application should quit based on key combination
pub fn should_quit(key_event: KeyEvent) -> bool {
    matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Job, JobStatus};
    
    #[test]
    fn test_should_quit() {
        let quit_key = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        assert!(should_quit(quit_key));

        let normal_key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        assert!(!should_quit(normal_key));
    }
}
