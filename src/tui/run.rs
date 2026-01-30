//! TUI runner - event loop and terminal management

use super::app::App;
use super::ui;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;

/// Run the TUI application
pub fn run(db_path: &str) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load initial data
    let mut app = App::new(db_path);
    if let Err(e) = app.refresh_data() {
        app.error_message = Some(format!("Failed to load data: {}", e));
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Main application loop
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Handle events with timeout for potential updates
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events, not releases
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
                        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => app.prev_tab(),
                        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                        KeyCode::Char('r') => {
                            if let Err(e) = app.refresh_data() {
                                app.error_message = Some(format!("Refresh failed: {}", e));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: TUI tests are difficult because they require a terminal
    // These are basic smoke tests

    #[test]
    fn test_app_creation_for_tui() {
        let app = App::new(":memory:");
        assert!(!app.should_quit);
    }
}
