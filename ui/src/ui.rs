use crate::types::{AppState, ViewMode, Job, JobStatus, LogLevel};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph,
        Wrap, BorderType,
    },
    Frame,
};

/// Main UI rendering function
pub fn draw(f: &mut Frame, app_state: &mut AppState) {
    let size = f.size();

    match app_state.current_view {
        ViewMode::Home => draw_home_view(f, size, app_state),
        ViewMode::JobDetail(job_id) => draw_job_detail_view(f, size, app_state, job_id),
        ViewMode::ConflictResolution(job_id) => draw_conflict_view(f, size, app_state, job_id),
        ViewMode::Help => draw_help_view(f, size),
        ViewMode::Settings => draw_settings_view(f, size, app_state),
    }
}

/// Draw the home view with job list and status
fn draw_home_view(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Job list
            Constraint::Length(4), // Footer with help
        ])
        .split(area);

    // Header
    draw_header(f, chunks[0], app_state);

    // Job list
    draw_job_list(f, chunks[1], app_state);

    // Footer
    draw_footer(f, chunks[2], "Home");
}

/// Draw the job detail view
fn draw_job_detail_view(f: &mut Frame, area: Rect, app_state: &AppState, job_id: uuid::Uuid) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(8), // Job details
            Constraint::Min(5),    // Action log
            Constraint::Length(4), // Footer
        ])
        .split(area);

    // Header
    draw_header(f, chunks[0], app_state);

    if let Some(job) = app_state.jobs.get(&job_id) {
        // Job details
        draw_job_details(f, chunks[1], job);

        // Action log
        draw_action_log(f, chunks[2], app_state, job_id);
    } else {
        let error_block = Paragraph::new("Job not found")
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .style(Style::default().fg(Color::Red));
        f.render_widget(error_block, chunks[1]);
    }

    // Footer
    draw_footer(f, chunks[3], "Job Detail");
}

/// Draw conflict resolution view
fn draw_conflict_view(f: &mut Frame, area: Rect, app_state: &AppState, job_id: uuid::Uuid) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Conflict list
            Constraint::Length(4), // Footer
        ])
        .split(area);

    // Header
    draw_header(f, chunks[0], app_state);

    if let Some(job) = app_state.jobs.get(&job_id) {
        draw_conflict_list(f, chunks[1], job, app_state.selected_conflict_index);
    } else {
        let error_block = Paragraph::new("Job not found")
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .style(Style::default().fg(Color::Red));
        f.render_widget(error_block, chunks[1]);
    }

    // Footer
    draw_footer(f, chunks[2], "Conflict Resolution");
}

/// Draw help view
fn draw_help_view(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Sync Dashboard Help", Style::default().add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/k      - Move up"),
        Line::from("  ↓/j      - Move down"),
        Line::from("  Enter    - Select/Enter detail view"),
        Line::from("  Esc/q    - Go back/Quit"),
        Line::from("  h/?      - Show this help"),
        Line::from(""),
        Line::from("Job Control:"),
        Line::from("  s        - Start selected job"),
        Line::from("  p        - Pause selected job"),
        Line::from("  t        - Stop selected job"),
        Line::from("  r        - Retry failed job"),
        Line::from(""),
        Line::from("Views:"),
        Line::from("  1        - Home view"),
        Line::from("  2        - Settings"),
        Line::from(""),
        Line::from("Conflict Resolution:"),
        Line::from("  1        - Keep source"),
        Line::from("  2        - Keep target"),
        Line::from("  3        - Merge"),
        Line::from("  4        - Skip"),
        Line::from(""),
        Line::from("Press any key to return..."),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_type(BorderType::Rounded)
        )
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, area);
}

/// Draw settings view
fn draw_settings_view(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Settings content
            Constraint::Length(4), // Footer
        ])
        .split(area);

    // Header
    draw_header(f, chunks[0], app_state);

    let settings_text = vec![
        Line::from(vec![
            Span::styled("Settings", Style::default().add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(format!("WebSocket Status: {}", 
            if app_state.websocket_connected { "Connected" } else { "Disconnected" }
        )),
        Line::from(format!("Total Jobs: {}", app_state.jobs.len())),
        Line::from(format!("Last Update: {}", app_state.last_update.format("%Y-%m-%d %H:%M:%S UTC"))),
        Line::from(""),
        Line::from("Configuration:"),
        Line::from("  PocketBase URL: http://localhost:8090"),
        Line::from("  Auto-refresh: Enabled"),
        Line::from("  Logging Level: Info"),
    ];

    let settings_paragraph = Paragraph::new(settings_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Settings")
                .border_type(BorderType::Rounded)
        )
        .wrap(Wrap { trim: true });

    f.render_widget(settings_paragraph, chunks[1]);

    // Footer
    draw_footer(f, chunks[2], "Settings");
}

/// Draw the header with connection status
fn draw_header(f: &mut Frame, area: Rect, app_state: &AppState) {
    let status_color = if app_state.websocket_connected {
        Color::Green
    } else {
        Color::Red
    };

    let status_text = if app_state.websocket_connected {
        "● Connected"
    } else {
        "● Disconnected"
    };

    let header_spans = vec![
        Span::styled("Sync Dashboard", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::raw(" | "),
        Span::raw(format!("Jobs: {}", app_state.jobs.len())),
    ];

    let header = Paragraph::new(Line::from(header_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Sync Dashboard")
                .border_type(BorderType::Rounded)
        )
        .alignment(Alignment::Center);

    f.render_widget(header, area);
}

/// Draw the job list with progress bars
fn draw_job_list(f: &mut Frame, area: Rect, app_state: &AppState) {
    let jobs = app_state.get_jobs_sorted();
    
    if jobs.is_empty() {
        let empty_message = Paragraph::new("No jobs available. Press 'n' to create a new job.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Jobs")
                    .border_type(BorderType::Rounded)
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(empty_message, area);
        return;
    }

    let job_items: Vec<ListItem> = jobs
        .iter()
        .enumerate()
        .map(|(i, job)| {
            let status_color = match job.status {
                JobStatus::Running => Color::Green,
                JobStatus::Paused => Color::Yellow,
                JobStatus::Failed => Color::Red,
                JobStatus::Completed => Color::Blue,
                JobStatus::Cancelled => Color::Gray,
                JobStatus::Pending => Color::Cyan,
            };

            let progress_bar = create_progress_bar(job.progress, 20);
            let files_info = job.files_progress_text();
            let bytes_info = job.bytes_progress_text();

            let style = if i == app_state.selected_job_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(format!("● {}", job.name), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" [{}]", job.status)),
                ]),
                Line::from(vec![
                    Span::raw(format!("  {} → {}", truncate_path(&job.source_path, 25), truncate_path(&job.target_path, 25))),
                ]),
                Line::from(vec![
                    Span::raw(format!("  Progress: {} {}% | Files: {} | Size: {}", 
                        progress_bar, job.progress_percentage(), files_info, bytes_info)),
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let job_list = List::new(job_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Jobs")
                .border_type(BorderType::Rounded)
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray));

    f.render_widget(job_list, area);
}

/// Draw detailed job information
fn draw_job_details(f: &mut Frame, area: Rect, job: &Job) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left panel - Basic info
    let left_content = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&job.name),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}", job.status), get_status_style(&job.status)),
        ]),
        Line::from(vec![
            Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", job.priority)),
        ]),
        Line::from(vec![
            Span::styled("Progress: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}%", job.progress_percentage())),
        ]),
        Line::from(vec![
            Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(job.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]),
    ];

    let left_panel = Paragraph::new(left_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Job Information")
                .border_type(BorderType::Rounded)
        )
        .wrap(Wrap { trim: true });

    // Right panel - Paths and progress
    let right_content = vec![
        Line::from(vec![
            Span::styled("Source: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&job.source_path),
        ]),
        Line::from(vec![
            Span::styled("Target: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&job.target_path),
        ]),
        Line::from(vec![
            Span::styled("Files: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(job.files_progress_text()),
        ]),
        Line::from(vec![
            Span::styled("Size: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(job.bytes_progress_text()),
        ]),
        Line::from(vec![
            Span::styled("Conflicts: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{}", job.conflicts.len()),
                if job.conflicts.is_empty() { 
                    Style::default().fg(Color::Green)
                } else { 
                    Style::default().fg(Color::Red)
                }
            ),
        ]),
    ];

    let right_panel = Paragraph::new(right_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Paths & Progress")
                .border_type(BorderType::Rounded)
        )
        .wrap(Wrap { trim: true });

    f.render_widget(left_panel, chunks[0]);
    f.render_widget(right_panel, chunks[1]);
}

/// Draw action log for a job
fn draw_action_log(f: &mut Frame, area: Rect, app_state: &AppState, job_id: uuid::Uuid) {
    let empty_logs = Vec::new();
    let logs = app_state.action_logs.get(&job_id).unwrap_or(&empty_logs);
    
    if logs.is_empty() {
        let empty_log = Paragraph::new("No log entries yet.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Action Log")
                    .border_type(BorderType::Rounded)
            )
            .alignment(Alignment::Center);
        f.render_widget(empty_log, area);
        return;
    }

    let log_items: Vec<ListItem> = logs
        .iter()
        .enumerate()
        .rev() // Show newest first
        .map(|(i, entry)| {
            let level_color = match entry.level {
                LogLevel::Error => Color::Red,
                LogLevel::Warning => Color::Yellow,
                LogLevel::Info => Color::Green,
                LogLevel::Debug => Color::Gray,
            };

            let style = if i == app_state.selected_log_index {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}]", entry.level),
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD)
                ),
                Span::raw(format!(" {} ", entry.timestamp.format("%H:%M:%S"))),
                Span::raw(format!("{}: {}", entry.action, entry.message)),
            ]);

            ListItem::new(content).style(style)
        })
        .collect();

    let log_list = List::new(log_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Action Log")
                .border_type(BorderType::Rounded)
        );

    f.render_widget(log_list, area);
}

/// Draw conflict list for resolution
fn draw_conflict_list(f: &mut Frame, area: Rect, job: &Job, selected_index: usize) {
    if job.conflicts.is_empty() {
        let no_conflicts = Paragraph::new("No conflicts to resolve.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Conflicts")
                    .border_type(BorderType::Rounded)
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green));
        f.render_widget(no_conflicts, area);
        return;
    }

    let conflict_items: Vec<ListItem> = job.conflicts
        .iter()
        .enumerate()
        .map(|(i, conflict)| {
            let style = if i == selected_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let resolution_text = match &conflict.resolution {
                Some(res) => format!(" [{}]", res),
                None => " [Pending]".to_string(),
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(format!("● {}", conflict.conflict_type), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled(resolution_text, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw(format!("  File: {}", truncate_path(&conflict.file_path, 60))),
                ]),
                Line::from(vec![
                    Span::raw(format!("  Source: {} | Target: {}", 
                        conflict.source_modified.format("%Y-%m-%d %H:%M:%S"),
                        conflict.target_modified.format("%Y-%m-%d %H:%M:%S")
                    )),
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let conflict_list = List::new(conflict_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Conflicts ({}/{})", 
                    job.conflicts.iter().filter(|c| c.resolution.is_some()).count(),
                    job.conflicts.len()
                ))
                .border_type(BorderType::Rounded)
        );

    f.render_widget(conflict_list, area);
}

/// Draw footer with key bindings
fn draw_footer(f: &mut Frame, area: Rect, view: &str) {
    let help_text = match view {
        "Home" => "↑/↓: Navigate | Enter: Details | s: Start | p: Pause | t: Stop | r: Retry | h: Help | q: Quit",
        "Job Detail" => "↑/↓: Navigate Log | Esc: Back | s: Start | p: Pause | t: Stop | r: Retry | c: Conflicts | q: Quit",
        "Conflict Resolution" => "↑/↓: Navigate | 1-4: Resolve | Esc: Back | q: Quit",
        "Settings" => "Esc: Back | q: Quit",
        _ => "h: Help | q: Quit",
    };

    let footer = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Controls")
                .border_type(BorderType::Rounded)
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(footer, area);
}

/// Helper function to create a simple progress bar
fn create_progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64) as usize;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Helper function to get status color style
fn get_status_style(status: &JobStatus) -> Style {
    match status {
        JobStatus::Running => Style::default().fg(Color::Green),
        JobStatus::Paused => Style::default().fg(Color::Yellow),
        JobStatus::Failed => Style::default().fg(Color::Red),
        JobStatus::Completed => Style::default().fg(Color::Blue),
        JobStatus::Cancelled => Style::default().fg(Color::Gray),
        JobStatus::Pending => Style::default().fg(Color::Cyan),
    }
}

/// Helper function to truncate long paths
fn truncate_path(path: &str, max_length: usize) -> String {
    if path.len() <= max_length {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_length + 3..])
    }
}
