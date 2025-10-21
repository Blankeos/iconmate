use std::sync::mpsc::sync_channel;

use crate::app_state::{App, AppFocus};
use crate::utils::{IconEntry, popup_area};
use ratatui::Frame;
use ratatui::crossterm::cursor::SetCursorStyle;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use tui_textarea::{Input, Key, TextArea};

#[derive(Debug)]
pub struct AddPopupState {
    pub inputs: Vec<TextArea<'static>>,
    pub current_input: usize,
}
impl AddPopupState {
    fn sync_cursor(&mut self, index: usize) {
        for (_i, textarea) in self.inputs.iter_mut().enumerate() {
            // Stay
            if index == _i {
                textarea.set_cursor_style(Style::default().bg(Color::White));
                continue;
            }
            // Remove
            textarea.set_cursor_style(Style::default());
        }
    }
}

impl App {
    pub fn init_add_popup(&mut self) {
        self.app_focus = AppFocus::AddPopup;
        self.add_popup_state = Some(AddPopupState {
            inputs: vec![
                TextArea::default(), // folder
                TextArea::default(), // preset
                TextArea::default(), // icon
                TextArea::default(), // filename
                TextArea::default(), // name
            ],
            current_input: 0,
        });

        // Set default value for folder input
        self.add_popup_state.as_mut().unwrap().inputs[0].insert_str("src/assets/icons");
        self.add_popup_state.as_mut().unwrap().sync_cursor(0);
    }

    pub fn handlekeys_add_popup(&mut self, input: Input) {
        if let Some(state) = self.add_popup_state.as_mut() {
            match input.key {
                Key::Enter => {
                    // Save current input
                    let current_text = state.inputs[state.current_input].lines().join("\n");
                    if state.current_input < state.inputs.len() - 1 {
                        state.current_input += 1;
                        state.sync_cursor(state.current_input);
                    } else {
                        // Submit form
                        let name = state.inputs[4].lines().join("\n");
                        let file_path = state.inputs[2].lines().join("\n");
                        self.items.push(IconEntry { name, file_path });
                        self.app_focus = AppFocus::Main;
                        self.add_popup_state = None;
                    }
                }
                Key::Tab => {
                    let mut forwards: i32 = 1;
                    if input.shift {
                        forwards = -1;
                    }
                    // Cycle
                    state.current_input = ((state.current_input as i32 + forwards)
                        .rem_euclid(state.inputs.len() as i32))
                        as usize;
                    state.sync_cursor(state.current_input);
                }
                Key::Esc => {
                    self.app_focus = AppFocus::Main;
                    self.add_popup_state = None;
                }
                _ => {
                    state.inputs[state.current_input].input(input);
                    // state.inputs[state.current_input].input
                }
            }
        }
    }
}

pub fn render_add_popup(f: &mut Frame, app: &mut App) {
    let area = popup_area(f.area(), 60, 50);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Folder
            Constraint::Length(3), // Preset
            Constraint::Length(3), // Icon
            Constraint::Length(3), // Filename
            Constraint::Length(3), // Name
            Constraint::Min(0),    // Help text
        ])
        .split(area);

    let title = Block::bordered()
        .title("Add Icon")
        .border_type(ratatui::widgets::BorderType::Rounded);
    f.render_widget(title, area);

    if let Some(state) = &mut app.add_popup_state {
        let labels = [
            "Folder",
            "Preset",
            "Icon (Iconify name / URL / raw SVG)",
            "Filename",
            "Name",
        ];

        // Render each field individually with textarea
        let folder_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[0]);
        state.inputs[0].set_block(folder_block);
        f.render_widget(state.inputs[0].widget(), layout[1]);

        let preset_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[1]);
        state.inputs[1].set_block(preset_block);
        f.render_widget(state.inputs[1].widget(), layout[2]);

        let icon_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 2 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[2]);
        state.inputs[2].set_block(icon_block);
        f.render_widget(state.inputs[2].widget(), layout[3]);

        let filename_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 3 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[3]);
        state.inputs[3].set_block(filename_block);
        f.render_widget(state.inputs[3].widget(), layout[4]);

        let name_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 4 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[4]);
        state.inputs[4].set_block(name_block);
        f.render_widget(state.inputs[4].widget(), layout[5]);
    }

    let help_text = Paragraph::new("Type and press Enter to continue | ESC to cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_text, layout[6]);
}
