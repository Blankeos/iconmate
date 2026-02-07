use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
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
    let area = popup_area(f.area(), 72, 17);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
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

    let config_lines = vec![
        Line::from(vec![
            Span::styled(
                "• ",
                Style::default().fg(if app.config.global_config_loaded {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled("Global Config", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(
                "• ",
                Style::default().fg(if app.config.project_config_loaded {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled("Project Config", Style::default().fg(Color::White)),
        ]),
    ];
    let config_status = Paragraph::new(config_lines).alignment(Alignment::Left);
    f.render_widget(config_status, layout[0]);

    let status = Paragraph::new(format!(
        " Folder: {}\n✦ Preset: {}\n[O] Viewer source: {}",
        app.config.folder,
        app.config.preset.as_deref().unwrap_or("<none>"),
        app.config.svg_viewer_cmd_source
    ))
    .alignment(Alignment::Left)
    .style(Style::default().fg(Color::White));
    f.render_widget(status, layout[1]);

    let divider = Paragraph::new(
        "a Add | i Iconify Search | d Delete | r Rename | o Open | / Search | ? Help | q Quit",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(divider, layout[3]);

    let nav = Paragraph::new("r Rename file path (alias stays the same)")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(nav, layout[4]);

    let ide_tip = Paragraph::new("Need to rename the icon symbol? Use your IDE Rename Symbol.")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(ide_tip, layout[5]);

    let help_text = Paragraph::new("Up/Down or j/k to navigate | Esc or ? to close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_text, layout[6]);
}
