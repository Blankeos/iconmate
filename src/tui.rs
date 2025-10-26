use crate::{
    app_state::{App, AppConfig, AppFocus},
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

pub async fn run(config: AppConfig) -> Result<(), anyhow::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);

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
    let width = area.width;

    let (direction, constraints) = if width < 80 {
        (
            ratatui::layout::Direction::Vertical,
            vec![Constraint::Min(3), Constraint::Min(0)],
        )
    } else {
        (
            ratatui::layout::Direction::Horizontal,
            vec![Constraint::Max(37), Constraint::Min(0)],
        )
    };

    let layout = ratatui::layout::Layout::default()
        .direction(direction)
        .margin(1)
        .constraints(constraints)
        .split(area);

    // Pages
    render_sidebar(f, layout[0], app);
    render_main_view(f, layout[1], app);

    // Modals

    match app.app_focus {
        AppFocus::AddPopup => crate::views::add_popup::render_add_popup(f, app),
        AppFocus::DeletePopup => crate::views::delete_popup::render_delete_popup(f, app),
        _ => {}
    }
}
