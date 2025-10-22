use std::sync::mpsc::sync_channel;

use crate::app_state::{App, AppFocus};
use crate::utils::{IconEntry, PRESETS_OPTIONS, Preset, PresetOption, popup_area};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};
use tui_textarea::{Input, Key, TextArea};

#[derive(Debug)]
pub struct AddPopupState {
    // Saved values
    folder: Option<String>,
    preset: Option<Preset>,
    icon: Option<String>,
    filename: Option<String>,
    name: Option<String>,

    // Form States
    pub current_input: usize,
    pub inputs: Vec<TextArea<'static>>,

    pub preset_index: usize,
    pub presets_filtered: Vec<PresetOption>,
    pub preset_filter: String,
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

    pub fn handlekeys_preset_input(&mut self, input: Input) {
        if self.current_input == 1 {
            match input.key {
                Key::Tab | Key::Enter => {
                    // Save the selected preset
                    if !self.presets_filtered.is_empty()
                        && self.preset_index < self.presets_filtered.len()
                    {
                        self.preset = Some(self.presets_filtered[self.preset_index].preset.clone());
                    }

                    self.current_input = (self.current_input + 1) % self.inputs.len();
                    self.sync_cursor(self.current_input);
                }
                Key::Up => {
                    let len = self.presets_filtered.len();
                    if len == 0 {
                        self.preset_index = 0;
                        return;
                    }
                    let delta = -1;
                    let new_index =
                        (self.preset_index as i32 + delta).rem_euclid(len as i32) as usize;
                    self.preset_index = new_index;
                }
                Key::Down => {
                    let len = self.presets_filtered.len();
                    if len == 0 {
                        self.preset_index = 0;
                        return;
                    }
                    let delta = 1;
                    let new_index =
                        (self.preset_index as i32 + delta).rem_euclid(len as i32) as usize;
                    self.preset_index = new_index;
                }
                _ => {
                    if self.current_input == 1 {
                        self.inputs[1].input(input);
                        self.preset_filter = self.inputs[1].lines().join("\n");

                        self.presets_filtered = PRESETS_OPTIONS
                            .iter()
                            .filter(|opt| {
                                let filter = self.preset_filter.to_lowercase();
                                filter.is_empty()
                                    || filter.contains(opt.preset.to_str())
                                    || opt.description.to_lowercase().contains(&filter)
                            })
                            .cloned()
                            .collect();
                        return;
                    }
                }
            }
        }
    }

    pub fn handlekeys_text_area(&mut self, input: Input) {
        match input.key {
            Key::Tab => {
                // Save the icon value
                self.icon = Some(self.inputs[2].lines().join("\n"));
                self.current_input = (self.current_input + 1) % self.inputs.len();
                self.sync_cursor(self.current_input);
            }
            Key::Char('v') if input.ctrl || input.alt => {
                // Cmd+V on macOS (alt+v in crossterm), Ctrl+V on Linux/Windows . ⌘
                if let Ok(mut ctx) = arboard::Clipboard::new() {
                    if let Ok(text) = ctx.get_text() {
                        self.inputs[self.current_input].insert_str(&text);
                    }
                }
            }
            _ => {
                self.inputs[self.current_input].input(input);
            }
        }
    }

    pub fn handlekeys_text_input(&mut self, input: Input) {
        match input.key {
            Key::Tab | Key::Enter => {
                // Save the current input value before moving to next
                let value = self.inputs[self.current_input].lines().join("");
                match self.current_input {
                    0 => self.folder = Some(value),   // folder
                    3 => self.filename = Some(value), // filename
                    4 => self.name = Some(value),     // name
                    _ => {}
                }
                self.current_input = (self.current_input + 1) % self.inputs.len();
                self.sync_cursor(self.current_input);
            }
            _ => {
                self.inputs[self.current_input].input(input);
            }
        }
    }
}

impl App {
    pub fn init_add_popup(&mut self) {
        self.app_focus = AppFocus::AddPopup;
        self.add_popup_state = Some(AddPopupState {
            folder: None,
            icon: None,
            name: None,
            preset: None,
            filename: None,

            preset_index: 0,
            preset_filter: String::new(),
            inputs: vec![
                TextArea::default(), // folder
                TextArea::default(), // preset (not used)
                TextArea::default(), // icon
                TextArea::default(), // filename
                TextArea::default(), // name
            ],
            presets_filtered: PRESETS_OPTIONS.to_vec(),
            current_input: 0,
        });

        // Set default value for folder input
        self.add_popup_state.as_mut().unwrap().inputs[0].insert_str("src/assets/icons");
        self.add_popup_state.as_mut().unwrap().sync_cursor(0);
    }

    pub fn handlekeys_add_popup(&mut self, input: Input) {
        if let Some(state) = self.add_popup_state.as_mut() {
            let _input = input.clone();

            match state.current_input {
                1 => state.handlekeys_preset_input(_input),
                2 => state.handlekeys_text_area(_input),
                _ => state.handlekeys_text_input(_input),
            }

            match input.key {
                Key::Esc => {
                    self.app_focus = AppFocus::Main;
                    self.add_popup_state = None;
                }
                _ => {}
            }
        }
    }
}

pub fn render_add_popup(f: &mut Frame, app: &mut App) {
    let area = popup_area(f.area(), 70, 30);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Folder
            Constraint::Length(7),  // Preset
            Constraint::Length(5),  // Icon
            Constraint::Length(3),  // Filename
            Constraint::Length(3),  // Name
            Constraint::Min(0),     // Help text
            Constraint::Length(20), //
        ])
        .split(area);

    let title = Block::bordered()
        .title("Add Icon")
        .border_type(ratatui::widgets::BorderType::Rounded);
    f.render_widget(title, area);

    if let Some(state) = &mut app.add_popup_state {
        let labels: Vec<String> = vec![
            String::from(" Folder"),
            if state.preset_filter.is_empty() {
                format!(" Preset")
            } else {
                format!(" Preset 🔍 {}", state.preset_filter)
            },
            String::from(" Icon (iconify name / URL / raw SVG / empty)"),
            String::from(" Filename"),
            String::from(" Name"),
        ];

        // Debug block - shows all saved values for development
        let debug_text = format!(
            "folder: {:?}\npreset: {:?}\nicon: {:?}\nfilename: {:?}\nname: {:?}",
            state.folder, state.preset, state.icon, state.filename, state.name
        );
        let debug_block = Block::default()
            .borders(Borders::ALL)
            .title("Debug Values")
            .style(Style::default().fg(Color::DarkGray));
        let debug_paragraph = Paragraph::new(debug_text)
            .block(debug_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(debug_paragraph, layout[7]);

        // Render each field individually with textarea
        let folder_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[0].clone());
        state.inputs[0].set_block(folder_block);
        state.inputs[0].set_cursor_line_style(Style::default());
        f.render_widget(&state.inputs[0], layout[1]);

        // Create a selectable list for preset
        let mut state_store = ratatui::widgets::ListState::default();
        let mut items: Vec<ListItem> = {
            let filtered: Vec<_> = state
                .presets_filtered
                .iter()
                .map(|p| {
                    ListItem::new(format!(
                        //  https://stackoverflow.com/questions/50458144/what-is-the-easiest-way-to-pad-a-string-with-0-to-the-left
                        "{:<8} - {}",
                        format!("{:?}", p.preset),
                        p.description
                    ))
                })
                .collect();
            filtered
        };
        if items.is_empty() {
            state_store.select(None);
            items =
                vec![ListItem::new("No presets found").style(Style::default().fg(Color::DarkGray))];
        } else {
            state_store.select(Some(state.preset_index))
        };
        let mut list = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(labels[1].clone())
                    .border_style(if state.current_input == 1 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_symbol("→ ");
        if state.current_input == 1 {
            list = list.highlight_style(Style::default().bg(Color::DarkGray))
        }

        f.render_stateful_widget(list, layout[2], &mut state_store);

        let icon_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 2 {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            })
            .title(labels[2].clone())
            .title_bottom(
                Line::from(if state.current_input == 2 {
                    "Tab to continue"
                } else {
                    ""
                })
                .alignment(Alignment::Right),
            )
            .title_bottom(
                Line::from(if state.current_input == 2 {
                    "ctrl+v to paste"
                } else {
                    ""
                })
                .alignment(Alignment::Left),
            );
        state.inputs[2].set_block(icon_block);
        state.inputs[2].set_cursor_line_style(Style::default());
        f.render_widget(&state.inputs[2], layout[3]);

        let filename_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 3 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[3].clone())
            .title(
                Line::from(crate::utils::filename_from_preset(
                    Some(state.inputs[3].lines().join("")),
                    state.preset.clone(),
                ))
                .alignment(Alignment::Right),
            );
        state.inputs[3].set_block(filename_block);
        state.inputs[3].set_cursor_line_style(Style::default());
        f.render_widget(&state.inputs[3], layout[4]);

        let name_value = state.inputs[4].lines().join("");
        let name_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.current_input == 4 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(format!("{}", labels[4]))
            .title(
                Line::from(if name_value.is_empty() {
                    String::from("usage: <Icon{} />")
                } else {
                    format!("usage: <Icon{} />", name_value)
                })
                .alignment(Alignment::Right),
            );
        state.inputs[4].set_block(name_block);
        state.inputs[4].set_cursor_line_style(Style::default());
        f.render_widget(&state.inputs[4], layout[5]);
    }

    let help_text = Paragraph::new("Tab/Enter to Continue | ESC to cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_text, layout[6]);
}
