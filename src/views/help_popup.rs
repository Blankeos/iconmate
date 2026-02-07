use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
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
    use ratatui::style::Modifier;

    let area = popup_area(f.area(), 76, 15);
    let body_area = crate::views::theme::render_popup_shell(f, area, "Help");

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(4),
        ])
        .split(body_area);

    let config_lines = vec![
        Line::from(vec![
            Span::styled(
                if app.config.global_config_loaded {
                    "● "
                } else {
                    "○ "
                },
                Style::default().fg(if app.config.global_config_loaded {
                    crate::views::theme::ACCENT
                } else {
                    crate::views::theme::SUBTLE_TEXT
                }),
            ),
            Span::styled(
                "Global config",
                Style::default()
                    .fg(crate::views::theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                if app.config.project_config_loaded {
                    "● "
                } else {
                    "○ "
                },
                Style::default().fg(if app.config.project_config_loaded {
                    crate::views::theme::ACCENT
                } else {
                    crate::views::theme::SUBTLE_TEXT
                }),
            ),
            Span::styled(
                "Local config",
                Style::default()
                    .fg(crate::views::theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let config_status = Paragraph::new(config_lines).alignment(Alignment::Left);
    f.render_widget(config_status, layout[0]);

    let svg_viewer_cmd = app.config.svg_viewer_cmd.as_deref().unwrap_or("Not set");
    let status = Paragraph::new(format!(
        "Folder: {}\nPreset: {}\nSVG viewer cmd: {}\nViewer cmd source: {}",
        app.config.folder,
        app.config.preset.as_str(),
        svg_viewer_cmd,
        app.config.svg_viewer_cmd_source
    ))
    .alignment(Alignment::Left)
    .style(Style::default().fg(crate::views::theme::MUTED_TEXT));
    f.render_widget(status, layout[2]);
}
