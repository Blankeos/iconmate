use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use tui_textarea::{Input, Key};

use crate::app_state::{App, AppFocus};
use crate::sync::{self, SyncPlan};
use crate::utils::popup_area;
use crate::views::theme;

#[derive(Debug)]
pub enum SyncPopupState {
    Plan(SyncPlan),
    Error(String),
}

impl App {
    pub fn init_sync_popup(&mut self) {
        let state = match build_sync_plan(&self.config) {
            Ok(plan) => SyncPopupState::Plan(plan),
            Err(err) => SyncPopupState::Error(format!("Failed to compute sync plan:\n{err}")),
        };
        self.sync_popup_state = Some(state);
        self.app_focus = AppFocus::SyncPopup;
    }

    pub fn handlekeys_sync_popup(&mut self, input: Input) {
        match input.key {
            Key::Esc | Key::Char('q') | Key::Char('S') | Key::Char('s') => {
                self.sync_popup_state = None;
                self.app_focus = AppFocus::Main;
            }
            _ => {}
        }
    }
}

fn build_sync_plan(
    config: &crate::app_state::AppConfig,
) -> anyhow::Result<SyncPlan> {
    let folder = PathBuf::from(&config.folder);
    let template = config
        .template
        .clone()
        .unwrap_or_else(|| crate::config::DEFAULT_OUTPUT_LINE_TEMPLATE.to_string());
    let barrel_file = config.flutter_barrel_file.as_deref().map(Path::new);
    let renames: HashMap<String, String> = HashMap::new();

    let ctx = sync::SyncContext {
        folder: &folder,
        preset: &config.preset,
        output_line_template: &template,
        flutter_barrel_file: barrel_file,
        flutter_barrel_class: config.flutter_barrel_class.as_deref(),
        renames: &renames,
    };
    sync::compute_sync_plan(&ctx)
}

pub fn render_sync_popup(f: &mut Frame, app: &App) {
    let area = popup_area(f.area(), 84, 24);
    let body_area = theme::render_popup_shell(f, area, "Sync");

    let state = match app.sync_popup_state.as_ref() {
        Some(s) => s,
        None => return,
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(body_area);

    let description = Paragraph::new(vec![
        Line::from(Span::styled(
            "Checks drift if you edited your icons folder manually without iconmate.",
            Style::default().fg(theme::MUTED_TEXT),
        )),
        Line::from(Span::styled(
            "Read-only in the TUI — run `iconmate sync --apply` to write.",
            Style::default().fg(theme::SUBTLE_TEXT),
        )),
    ])
    .alignment(Alignment::Left);
    f.render_widget(description, layout[0]);

    match state {
        SyncPopupState::Error(msg) => {
            let body = Paragraph::new(msg.clone())
                .style(Style::default().fg(theme::ERROR))
                .alignment(Alignment::Left);
            f.render_widget(body, layout[2]);
        }
        SyncPopupState::Plan(plan) => {
            let body = Paragraph::new(plan_to_lines(plan)).alignment(Alignment::Left);
            f.render_widget(body, layout[2]);
        }
    }
}

const ADD_COLOR: Color = theme::ACCENT;
const PRUNE_COLOR: Color = theme::ERROR;
const WARN_COLOR: Color = Color::Rgb(250, 204, 21);

fn plan_to_lines(plan: &SyncPlan) -> Vec<Line<'static>> {
    let text = Style::default().fg(theme::TEXT);
    let muted = Style::default().fg(theme::MUTED_TEXT);
    let subtle = Style::default().fg(theme::SUBTLE_TEXT);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Barrel: ", muted),
        Span::styled(plan.barrel_location.clone(), text),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Preset: ", muted),
        Span::styled(plan.preset.clone(), text),
    ]));
    lines.push(Line::from(""));

    if plan.is_clean() {
        lines.push(Line::from(Span::styled(
            "● It's clean and synced!",
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )));
        return lines;
    }

    if !plan.additions.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Would add ({}):", plan.additions.len()),
            Style::default().fg(ADD_COLOR).add_modifier(Modifier::BOLD),
        )));
        for a in &plan.additions {
            lines.push(Line::from(vec![
                Span::styled(format!("  + {:<24}", a.identifier), Style::default().fg(ADD_COLOR)),
                Span::styled(" → ", muted),
                Span::styled(a.file_path.clone(), Style::default().fg(ADD_COLOR)),
                Span::styled("  (orphan file)", subtle),
            ]));
        }
        lines.push(Line::from(""));
    }

    if !plan.removals.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Would prune ({}):", plan.removals.len()),
            Style::default().fg(PRUNE_COLOR).add_modifier(Modifier::BOLD),
        )));
        for r in &plan.removals {
            lines.push(Line::from(vec![
                Span::styled(format!("  - {:<24}", r.identifier), Style::default().fg(PRUNE_COLOR)),
                Span::styled(" → ", muted),
                Span::styled(r.file_path.clone(), Style::default().fg(PRUNE_COLOR)),
                Span::styled("  (file missing)", subtle),
            ]));
        }
        lines.push(Line::from(""));
    }

    if !plan.collisions.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Collisions ({}):", plan.collisions.len()),
            Style::default().fg(WARN_COLOR).add_modifier(Modifier::BOLD),
        )));
        for c in &plan.collisions {
            lines.push(Line::from(vec![
                Span::styled(format!("  ! {}", c.inferred_identifier), Style::default().fg(WARN_COLOR)),
                Span::styled(" collides with ", muted),
                Span::styled(format!("`{}`", c.conflicting_identifier), Style::default().fg(WARN_COLOR)),
                Span::styled(format!(" (from {})", c.file_path), subtle),
            ]));
        }
        lines.push(Line::from(""));
    }

    if !plan.additions.is_empty() || !plan.removals.is_empty() {
        lines.push(Line::from(Span::styled(
            "Run with --apply to write additions.",
            muted,
        )));
        if !plan.removals.is_empty() {
            lines.push(Line::from(Span::styled(
                "Run with --apply --prune to also remove orphan entries.",
                muted,
            )));
        }
    }
    if !plan.collisions.is_empty() {
        lines.push(Line::from(Span::styled(
            "Resolve collisions with --rename <inferred>=<newName>, or rename the SVG on disk.",
            muted,
        )));
    }

    lines
}
