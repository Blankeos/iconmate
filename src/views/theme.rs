use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

pub const BASE_BG: Color = Color::Rgb(10, 14, 22);
pub const PANEL_BG: Color = Color::Reset;
pub const INPUT_BG: Color = Color::Reset;
pub const LOGO_GREEN: Color = Color::Rgb(74, 222, 128);
pub const TAB_BG: Color = Color::Reset;
pub const TAB_BG_ACTIVE: Color = LOGO_GREEN;
pub const ROW_HIGHLIGHT_BG: Color = LOGO_GREEN;

pub const TEXT: Color = Color::Rgb(236, 240, 246);
pub const MUTED_TEXT: Color = Color::Rgb(143, 155, 177);
pub const SUBTLE_TEXT: Color = Color::Rgb(110, 124, 149);
pub const ACCENT: Color = LOGO_GREEN;
pub const ACCENT_SOFT: Color = LOGO_GREEN;
pub const ERROR: Color = Color::Rgb(248, 113, 113);

pub fn render_popup_shell(f: &mut Frame, area: Rect, title: &str) -> Rect {
    f.render_widget(Clear, area);
    f.render_widget(
        Block::bordered().border_style(Style::default().fg(SUBTLE_TEXT)),
        area,
    );

    let frame = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title_width = title.chars().count().saturating_add(1) as u16;
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(title_width),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame[1]);

    f.render_widget(
        Paragraph::new(title).style(Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        header[1],
    );
    f.render_widget(
        Paragraph::new("esc")
            .style(
                Style::default()
                    .fg(ACCENT_SOFT)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Right),
        header[3],
    );

    let body_with_gap = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(frame[2]);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(body_with_gap[1])[1]
}

pub fn shortcut_line(items: &[(&str, &str)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    for (index, (label, key)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            (*label).to_string(),
            Style::default()
                .fg(ACCENT_SOFT)
                .add_modifier(Modifier::BOLD),
        ));
        if !key.is_empty() {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                (*key).to_string(),
                Style::default().fg(MUTED_TEXT),
            ));
        }
    }

    Line::from(spans)
}
