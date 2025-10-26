use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, ListItem};
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
            Key::Up | Key::Char('k') => {
                state.selected_index = state.selected_index.saturating_sub(1);
            }
            Key::Down | Key::Char('j') => {
                state.selected_index = (state.selected_index + 1).min(1);
            }
            _ => {}
        }
    }
}

pub fn render_delete_popup(f: &mut Frame, app: &mut App) {
    let area = popup_area(f.area(), 60, 12);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(0),    // List
            Constraint::Length(1), // Help
        ])
        .split(area);

    let title = Block::bordered()
        .title(format!("🗑 Delete Icon"))
        .title_style(Style::default().fg(Color::White))
        .border_type(ratatui::widgets::BorderType::Rounded);
    f.render_widget(title, area);

    if let Some(state) = &mut app.delete_popup_state {
        let items = vec![
            // ListItem::new(format!("y Delete this icon ({})", state.selected_item.name)),
            ListItem::new(format!(
                "y Delete the icon '{}'",
                state
                    .item_to_delete
                    .as_ref()
                    .map(|item| item.name.as_str())
                    .unwrap_or("Name")
            )),
            ListItem::new("n Cancel"),
        ];

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.selected_index));

        let list_block = ratatui::widgets::List::new(items)
            .block(Block::default())
            .highlight_style(if state.selected_index == 0 {
                Style::default().bg(Color::Red)
            } else {
                Style::default().bg(Color::DarkGray)
            })
            .highlight_symbol("→ ");

        f.render_stateful_widget(list_block, layout[0], &mut list_state);
    }

    let help_text =
        ratatui::widgets::Paragraph::new("y/n or j/k to select | Enter to confirm | Esc to cancel")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Gray));
    f.render_widget(help_text, layout[1]);
}
