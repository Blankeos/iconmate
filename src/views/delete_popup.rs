use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_textarea::{Input, Key};

#[derive(Debug)]
pub struct DeletePopupState {
    pub selected_index: usize, // For yes or no only

    pub item_to_delete: Option<crate::utils::IconEntry>,
}

impl App {
    pub fn init_delete_popup(&mut self) {
        self.app_focus = AppFocus::DeletePopup;

        if let Some(item_to_delete) = self.filtered_items.get(self.selected_index) {
            self.delete_popup_state = Some(DeletePopupState {
                selected_index: 0,
                item_to_delete: Some(item_to_delete.clone()),
            });
        } else {
            self.delete_popup_state = Some(DeletePopupState {
                selected_index: 0,
                item_to_delete: None,
            });
        }
    }

    fn close_delete_popup(&mut self) {
        self.app_focus = AppFocus::Main;
        self.delete_popup_state = None;
    }

    fn perform_delete_action(&mut self) {
        // Remove the item from the items vector
        if let Some(state) = &self.delete_popup_state {
            if let Some(item) = &state.item_to_delete {
                if let Some(pos) = self.items.iter().position(|i| i.name == item.name) {
                    self.items.remove(pos);
                }

                // Persist the change to disk
                let abs_file_path = std::path::Path::new(&self.config.folder).join(&item.file_path);
                if let Err(e) =
                    crate::utils::delete_icon_entry(abs_file_path.to_str().unwrap_or(""))
                {
                    eprintln!("Failed to delete icon file: {}", e);
                }
            }
        }

        // Re-initialize icons from disk to ensure consistency
        self.init_icons();
    }

    pub fn handlekeys_delete_popup(&mut self, input: Input) {
        let Some(state) = self.delete_popup_state.as_mut() else {
            return;
        };

        match input.key {
            Key::Char('y') => {
                // Perform delete action
                self.perform_delete_action();
                self.close_delete_popup();
            }
            Key::Char('n') | Key::Esc => {
                // Cancel
                self.close_delete_popup();
            }
            Key::Enter => {
                if state.selected_index == 0 {
                    // Perform delete action
                    self.perform_delete_action();
                    self.close_delete_popup();
                } else {
                    // Cancel if "n" is selected
                    self.close_delete_popup();
                }
            }
            Key::Left | Key::Char('h') | Key::Up | Key::Char('k') => {
                state.selected_index = state.selected_index.saturating_sub(1);
            }
            Key::Right | Key::Char('l') | Key::Down | Key::Char('j') => {
                state.selected_index = (state.selected_index + 1).min(1);
            }
            _ => {}
        }
    }
}

pub fn render_delete_popup(f: &mut Frame, app: &mut App) {
    use ratatui::{
        style::Modifier,
        text::{Line, Span},
    };

    let area = popup_area(f.area(), 58, 10);
    let body_area = crate::views::theme::render_popup_shell(f, area, "Delete Icon");

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(body_area);

    if let Some(state) = &mut app.delete_popup_state {
        let icon_name = state
            .item_to_delete
            .as_ref()
            .map(|item| item.name.as_str())
            .unwrap_or("this icon");
        let prompt = Paragraph::new(format!("Delete '{icon_name}'?"))
            .alignment(Alignment::Left)
            .style(
                Style::default()
                    .fg(crate::views::theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(prompt, layout[0]);

        let action_line = if state.selected_index == 0 {
            Line::from(vec![
                Span::styled(
                    " Delete ",
                    Style::default()
                        .bg(crate::views::theme::ERROR)
                        .fg(crate::views::theme::BASE_BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled("y", Style::default().fg(crate::views::theme::MUTED_TEXT)),
                Span::raw("     "),
                Span::styled(
                    "Cancel",
                    Style::default()
                        .fg(crate::views::theme::SUBTLE_TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled("n", Style::default().fg(crate::views::theme::MUTED_TEXT)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "Delete",
                    Style::default()
                        .fg(crate::views::theme::ACCENT_SOFT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled("y", Style::default().fg(crate::views::theme::MUTED_TEXT)),
                Span::raw("     "),
                Span::styled(
                    " Cancel ",
                    Style::default()
                        .bg(crate::views::theme::ROW_HIGHLIGHT_BG)
                        .fg(crate::views::theme::BASE_BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled("n", Style::default().fg(crate::views::theme::MUTED_TEXT)),
            ])
        };
        f.render_widget(
            Paragraph::new(action_line).alignment(Alignment::Left),
            layout[2],
        );
    }
}
