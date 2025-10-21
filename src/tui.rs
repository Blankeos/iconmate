use crate::{
    app_state::{App, AppFocus},
    views::main::{render_main_view, render_sidebar},
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend, layout::Constraint};
use std::io;
use tui_textarea::{Input, Key};

pub async fn run() -> Result<(), anyhow::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        match ratatui::crossterm::event::read()?.into() {
            Input {
                key: Key::Char('q'),
                ..
            } => break,
            input => app.handlekeys(input),
        }

        app.update();
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Max(37), Constraint::Min(0)])
        .split(area);

    render_sidebar(f, layout[0], app);
    render_main_view(f, layout[1], app);

    if app.app_focus == AppFocus::AddPopup {
        crate::views::add_popup::render_add_popup(f, app);
    }
}
