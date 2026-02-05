use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};
use tui_textarea::{Input, Key};

impl App {
    pub fn init_help_popup(&mut self) {
        self.app_focus = AppFocus::HelpPopup;
    }

    pub fn handlekeys_help_popup(&mut self, input: Input) {
        match input.key {
            Key::Esc | Key::Char('?') => {
                self.app_focus = AppFocus::Main;
            }
            _ => {}
        }
    }
}

pub fn render_help_popup(f: &mut Frame, app: &App) {
    let area = popup_area(f.area(), 62, 12);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let title = Block::bordered()
        .title("Help")
        .title_style(Style::default().fg(Color::White))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title_alignment(Alignment::Center);
    f.render_widget(title, area);

    let status = Paragraph::new(format!(
        "Folder: {}\nPreset: {}",
        app.config.folder,
        app.config.preset.as_deref().unwrap_or("<none>")
    ))
    .alignment(Alignment::Left)
    .style(Style::default().fg(Color::White));
    f.render_widget(status, layout[0]);

    let divider = Paragraph::new("a Add | d Delete | / Search | ? Help | q Quit")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(divider, layout[2]);

    let nav = Paragraph::new("Up/Down or j/k to navigate")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(nav, layout[3]);

    let help_text = Paragraph::new("Esc or ? to close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_text, layout[4]);
}
