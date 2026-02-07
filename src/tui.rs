use crate::{
    app_state::{App, AppConfig, AppFocus},
    views::main::render_main_view,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend, layout::Constraint};
use std::{io, time::Duration};
use tui_textarea::{Input, Key};

struct TerminalCleanupGuard;

impl Drop for TerminalCleanupGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
    }
}

pub async fn run(config: AppConfig) -> Result<(), anyhow::Error> {
    enable_raw_mode()?;
    let _cleanup = TerminalCleanupGuard;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if ratatui::crossterm::event::poll(Duration::from_millis(16))? {
            match ratatui::crossterm::event::read()?.into() {
                Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => break,
                input => app.handlekeys(input),
            }
        }

        app.update();
        if app.should_quit {
            break;
        }
    }

    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(area);

    // Pages
    render_main_view(f, layout[0], app);

    // Modals

    match app.app_focus {
        AppFocus::AddPopup => crate::views::add_popup::render_add_popup(f, app),
        AppFocus::DeletePopup => crate::views::delete_popup::render_delete_popup(f, app),
        AppFocus::RenamePopup => crate::views::rename_popup::render_rename_popup(f, app),
        AppFocus::HelpPopup => crate::views::help_popup::render_help_popup(f, app),
        AppFocus::IconifySearchPopup => {
            crate::views::iconify_search_popup::render_iconify_search_popup(f, app)
        }
        _ => {}
    }
}
