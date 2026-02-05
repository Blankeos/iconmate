use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
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
    let area = popup_area(f.area(), 64, 16);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Block::bordered()
        .title("Help")
        .title_style(Style::default().fg(Color::White))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title_alignment(Alignment::Center);
    f.render_widget(title, area);

    let defaults_items = vec![
        ListItem::new(format!("Folder: {}", app.config.folder)),
        ListItem::new(format!(
            "Preset: {}",
            app.config.preset.as_deref().unwrap_or("<none>")
        )),
    ];
    let defaults = List::new(defaults_items).block(
        Block::default()
            .title("Defaults")
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded),
    );
    f.render_widget(defaults, layout[0]);

    let key_items = vec![
        ListItem::new("a  Add icon"),
        ListItem::new("d  Delete icon"),
        ListItem::new("/  Search"),
        ListItem::new("?  Help"),
        ListItem::new("q  Quit"),
        ListItem::new("Up/Down or j/k  Navigate"),
    ];
    let keys = List::new(key_items).block(
        Block::default()
            .title("Keybinds")
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded),
    );
    f.render_widget(keys, layout[1]);

    let help_text = Paragraph::new("Esc or ? to close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_text, layout[2]);
}
