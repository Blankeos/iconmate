use std::path::Path;

use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use tui_textarea::{Input, Key, TextArea};

#[derive(Debug)]
pub struct RenamePopupState {
    pub item_to_rename: Option<crate::utils::IconEntry>,
    pub filename_input: TextArea<'static>,
    pub status_message: Option<String>,
    pub status_is_error: bool,
}

impl RenamePopupState {
    fn is_paste_shortcut(input: &Input) -> bool {
        matches!(input.key, Key::Char('v')) && (input.ctrl || input.alt)
    }

    fn paste_into_input(&mut self) -> bool {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                self.filename_input.insert_str(&text);
                return true;
            }
        }
        false
    }

    fn clear_status(&mut self) {
        self.status_message = None;
        self.status_is_error = false;
    }

    fn set_status_error(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_is_error = true;
    }
}

impl App {
    pub fn init_rename_popup(&mut self) {
        self.app_focus = AppFocus::RenamePopup;

        let item_to_rename = self.filtered_items.get(self.selected_index).cloned();
        let mut filename_input = TextArea::default();

        if let Some(item) = &item_to_rename {
            if let Some(file_name) = Path::new(&item.file_path)
                .file_name()
                .and_then(|name| name.to_str())
            {
                filename_input.insert_str(file_name);
            }
        }

        filename_input.set_cursor_style(Style::default().bg(Color::White));

        self.rename_popup_state = Some(RenamePopupState {
            item_to_rename,
            filename_input,
            status_message: None,
            status_is_error: false,
        });
    }

    fn close_rename_popup(&mut self) {
        self.app_focus = AppFocus::Main;
        self.rename_popup_state = None;
    }

    fn submit_rename_popup(&mut self) -> Result<(), String> {
        let Some(state) = self.rename_popup_state.as_ref() else {
            return Err("Rename popup is not initialized".to_string());
        };

        let Some(item) = state.item_to_rename.as_ref() else {
            return Err("No icon selected to rename.".to_string());
        };

        let new_filename = state.filename_input.lines().join("\n").trim().to_string();
        if new_filename.is_empty() {
            return Err("Please enter a new filename.".to_string());
        }

        crate::utils::rename_icon_entry(&self.config.folder, &item.file_path, &new_filename)
            .map_err(|error| error.to_string())?;

        self.init_icons();
        self.close_rename_popup();
        Ok(())
    }

    pub fn handlekeys_rename_popup(&mut self, input: Input) {
        match input.key {
            Key::Esc => {
                self.close_rename_popup();
            }
            Key::Enter => {
                if let Err(error) = self.submit_rename_popup() {
                    if let Some(state) = self.rename_popup_state.as_mut() {
                        state.set_status_error(error);
                    }
                }
            }
            _ => {
                if let Some(state) = self.rename_popup_state.as_mut() {
                    if RenamePopupState::is_paste_shortcut(&input) && state.paste_into_input() {
                        state.clear_status();
                        return;
                    }

                    state.filename_input.input(input);
                    state.clear_status();
                }
            }
        }
    }
}

pub fn render_rename_popup(f: &mut Frame, app: &mut App) {
    let area = popup_area(f.area(), 72, 12);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let title = Block::bordered()
        .title("Rename File")
        .title_style(Style::default().fg(Color::White))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title_alignment(Alignment::Center);
    f.render_widget(title, area);

    if let Some(state) = app.rename_popup_state.as_mut() {
        let status = if let Some(item) = &state.item_to_rename {
            format!("Alias: {}\nCurrent file: {}", item.name, item.file_path)
        } else {
            "No icon selected".to_string()
        };
        let status_paragraph = Paragraph::new(status)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status_paragraph, layout[0]);

        let input_block = Block::default()
            .borders(Borders::TOP)
            .title("New filename")
            .border_style(Style::default().fg(Color::Yellow));
        state.filename_input.set_block(input_block);
        state.filename_input.set_cursor_line_style(Style::default());
        f.render_widget(&state.filename_input, layout[1]);

        let tip = Paragraph::new(
            "Renames only the file path export target. For alias rename, use your IDE Rename Symbol.",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(tip, layout[2]);

        let footer_text = state
            .status_message
            .clone()
            .unwrap_or_else(|| "Enter to rename, Esc to cancel".to_string());
        let footer_color = if state.status_is_error {
            Color::Red
        } else {
            Color::DarkGray
        };
        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(footer_color));
        f.render_widget(footer, layout[3]);
    }
}
