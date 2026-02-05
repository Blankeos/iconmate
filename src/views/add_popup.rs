use std::process::Command;

use crate::app_state::{App, AppFocus};
use crate::utils::{PRESETS_OPTIONS, Preset, PresetOption, popup_area};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};
use tui_textarea::{Input, Key, TextArea};

// Constants
// const FOLDER_FIELD_IDX: usize = 0;
const PRESET_FIELD_IDX: usize = 0;
const ICON_FIELD_IDX: usize = 1;
const FILENAME_FIELD_IDX: usize = 2;
const NAME_FIELD_IDX: usize = 3;
const FOOTER_FIELD_IDX: usize = 4;
const DEFAULT_OUTPUT_LINE_TEMPLATE: &str = "export { default as Icon%name% } from './%icon%%ext%';";

#[derive(Debug)]
pub struct AddPopupState {
    // Saved values
    // folder: Option<String>,
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
    pub status_message: Option<String>,
    pub status_is_error: bool,
}
impl AddPopupState {
    fn is_paste_shortcut(input: &Input) -> bool {
        matches!(input.key, Key::Char('v')) && (input.ctrl || input.alt)
    }

    fn paste_into_current_input(&mut self) -> bool {
        if let Ok(mut ctx) = arboard::Clipboard::new() {
            if let Ok(text) = ctx.get_text() {
                self.inputs[self.current_input].insert_str(&text);
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

    fn apply_icon_based_defaults(&mut self) {
        let icon_raw = self.inputs[ICON_FIELD_IDX].lines().join("\n");
        let icon_source = icon_raw.trim();
        if icon_source.is_empty() {
            return;
        }

        if let Some((default_name, default_filename)) =
            crate::utils::default_name_and_filename_from_icon_source(icon_source)
        {
            let has_filename = !self.inputs[FILENAME_FIELD_IDX]
                .lines()
                .join("")
                .trim()
                .is_empty();
            if !has_filename {
                self.inputs[FILENAME_FIELD_IDX] = TextArea::default();
                self.inputs[FILENAME_FIELD_IDX].insert_str(&default_filename);
                self.filename = Some(default_filename);
            }

            let has_name = !self.inputs[NAME_FIELD_IDX]
                .lines()
                .join("")
                .trim()
                .is_empty();
            if !has_name {
                self.inputs[NAME_FIELD_IDX] = TextArea::default();
                self.inputs[NAME_FIELD_IDX].insert_str(&default_name);
                self.name = Some(default_name);
            }
        }
    }

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
        if self.current_input == PRESET_FIELD_IDX {
            if Self::is_paste_shortcut(&input) && self.paste_into_current_input() {
                self.preset_filter = self.inputs[PRESET_FIELD_IDX].lines().join("\n");
            }

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
                    if self.current_input == PRESET_FIELD_IDX {
                        if !Self::is_paste_shortcut(&input) {
                            self.inputs[PRESET_FIELD_IDX].input(input);
                        }
                        self.preset_filter = self.inputs[PRESET_FIELD_IDX].lines().join("\n");

                        self.presets_filtered = PRESETS_OPTIONS
                            .iter()
                            .filter(|opt| {
                                let filter = self.preset_filter.to_lowercase();
                                filter.is_empty()
                                    || opt.preset.to_str().contains(&filter)
                                    || opt.description.to_lowercase().contains(&filter)
                            })
                            .cloned()
                            .collect();
                        if self.preset_index >= self.presets_filtered.len() {
                            self.preset_index = 0;
                        }
                        self.clear_status();
                        return;
                    }
                }
            }
        }
    }

    pub fn handlekeys_text_area(&mut self, input: Input) {
        if Self::is_paste_shortcut(&input) && self.paste_into_current_input() {
            self.clear_status();
            return;
        }

        match input.key {
            Key::Tab => {
                // Save the icon value
                self.icon = Some(self.inputs[ICON_FIELD_IDX].lines().join("\n"));
                self.apply_icon_based_defaults();
                self.current_input = (self.current_input + 1) % self.inputs.len();
                self.sync_cursor(self.current_input);
            }
            _ => {
                self.inputs[self.current_input].input(input);
                self.clear_status();
            }
        }
    }

    pub fn handlekeys_text_input(&mut self, input: Input) {
        if Self::is_paste_shortcut(&input) && self.paste_into_current_input() {
            self.clear_status();
            return;
        }

        match input.key {
            Key::Tab | Key::Enter => {
                // Save the current input value before moving to next
                let value = self.inputs[self.current_input].lines().join("");
                match self.current_input {
                    // 0 => self.folder = Some(value),                    // folder
                    FILENAME_FIELD_IDX => self.filename = Some(value), // filename
                    NAME_FIELD_IDX => self.name = Some(value),         // name
                    _ => {}
                }
                self.current_input = (self.current_input + 1) % self.inputs.len();
                self.sync_cursor(self.current_input);
                self.clear_status();
            }
            _ => {
                self.inputs[self.current_input].input(input);
                self.clear_status();
            }
        }
    }
}

impl App {
    pub fn init_add_popup(&mut self) {
        let configured_preset = self.config.preset.as_ref().and_then(|preset_str| {
            PRESETS_OPTIONS
                .iter()
                .find(|option| option.preset.to_str() == preset_str)
                .map(|option| option.preset.clone())
        });
        let selected_index = configured_preset
            .as_ref()
            .and_then(|preset| {
                PRESETS_OPTIONS
                    .iter()
                    .position(|option| option.preset == *preset)
            })
            .unwrap_or(0);

        self.app_focus = AppFocus::AddPopup;
        self.add_popup_state = Some(AddPopupState {
            // folder: None,
            icon: None,
            name: None,
            preset: configured_preset,
            filename: None,

            preset_index: selected_index,
            preset_filter: String::new(),
            inputs: vec![
                TextArea::default(), // preset filter
                TextArea::default(), // icon
                TextArea::default(), // filename
                TextArea::default(), // name
            ],
            presets_filtered: PRESETS_OPTIONS.to_vec(),
            current_input: 0,
            status_message: None,
            status_is_error: false,
        });

        // Unused: Set default value for folder input
        // Unused: self.add_popup_state.as_mut().unwrap().inputs[0].insert_str(&self.config.folder);
        self.add_popup_state
            .as_mut()
            .unwrap()
            .sync_cursor(PRESET_FIELD_IDX); // The first.
    }

    fn submit_add_popup(&mut self) -> Result<(), String> {
        let (preset, icon, filename, name) = {
            let Some(state) = self.add_popup_state.as_mut() else {
                return Err("Add popup is not initialized".to_string());
            };

            state.apply_icon_based_defaults();

            if state.preset.is_none()
                && !state.presets_filtered.is_empty()
                && state.preset_index < state.presets_filtered.len()
            {
                state.preset = Some(state.presets_filtered[state.preset_index].preset.clone());
            }

            let preset = state.preset.clone();
            let icon = state.inputs[ICON_FIELD_IDX]
                .lines()
                .join("\n")
                .trim()
                .to_string();
            let filename = state.inputs[FILENAME_FIELD_IDX]
                .lines()
                .join("")
                .trim()
                .to_string();

            let mut name = state.inputs[NAME_FIELD_IDX]
                .lines()
                .join("")
                .trim()
                .to_string();
            if name.is_empty() {
                if let Some(icon_source) = (!icon.is_empty()).then_some(icon.as_str()) {
                    if let Some((default_name, _)) =
                        crate::utils::default_name_and_filename_from_icon_source(icon_source)
                    {
                        state.inputs[NAME_FIELD_IDX] = TextArea::default();
                        state.inputs[NAME_FIELD_IDX].insert_str(&default_name);
                        state.name = Some(default_name.clone());
                        name = default_name;
                    }
                }
            }

            (preset, icon, filename, name)
        };

        if name.is_empty() {
            return Err(
                "Name is required. Add one or use an Iconify icon URL/name for auto-fill."
                    .to_string(),
            );
        }

        if icon.is_empty() && preset.is_none() {
            return Err("Choose a preset or provide an icon source first.".to_string());
        }

        let mut command = Command::new(std::env::current_exe().map_err(|error| error.to_string())?);
        command
            .arg("add")
            .arg("--folder")
            .arg(&self.config.folder)
            .arg("--name")
            .arg(&name)
            .arg("--output-line-template")
            .arg(
                self.config
                    .template
                    .as_deref()
                    .unwrap_or(DEFAULT_OUTPUT_LINE_TEMPLATE),
            );

        if let Some(preset) = preset {
            command.arg("--preset").arg(preset.to_str());
        }

        if !icon.is_empty() {
            command.arg("--icon").arg(icon);
        }

        if !filename.is_empty() {
            command.arg("--filename").arg(filename);
        }

        let output = command.output().map_err(|error| error.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let message = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "Failed to add icon".to_string()
            };
            return Err(message);
        }

        self.init_icons();
        self.app_focus = AppFocus::Main;
        self.add_popup_state = None;

        Ok(())
    }

    pub fn handlekeys_add_popup(&mut self, input: Input) {
        let should_submit = self
            .add_popup_state
            .as_ref()
            .map(|state| state.current_input == NAME_FIELD_IDX && input.key == Key::Enter)
            .unwrap_or(false);

        if should_submit {
            if let Err(error) = self.submit_add_popup() {
                if let Some(state) = self.add_popup_state.as_mut() {
                    state.set_status_error(error);
                }
            }
            return;
        }

        if let Some(state) = self.add_popup_state.as_mut() {
            let _input = input.clone();

            match state.current_input {
                PRESET_FIELD_IDX => state.handlekeys_preset_input(_input),
                ICON_FIELD_IDX => state.handlekeys_text_area(_input),
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
    let area = popup_area(f.area(), 70, 24);
    f.render_widget(ratatui::widgets::Clear, area);

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(6), // Preset
            Constraint::Length(4), // Icon
            Constraint::Length(3), // Filename
            Constraint::Length(3), // Name
            Constraint::Min(0),    // Footer
        ])
        .split(area);

    let title = Block::bordered()
        .title("Add Icon")
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title_style(Style::default().fg(Color::White))
        .title_alignment(Alignment::Center);
    f.render_widget(title, area);

    if let Some(state) = &mut app.add_popup_state {
        let labels: Vec<String> = vec![
            if state.preset_filter.is_empty() {
                String::from("Preset")
            } else {
                format!("Preset filter: {}", state.preset_filter)
            },
            String::from("Icon source (name, URL, SVG, or empty)"),
            String::from("Filename"),
            String::from("Name"),
        ];

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
                    .borders(Borders::TOP)
                    .title(labels[PRESET_FIELD_IDX].clone())
                    .border_style(if state.current_input == PRESET_FIELD_IDX {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_symbol("→ ");
        if state.current_input == PRESET_FIELD_IDX {
            list = list.highlight_style(Style::default().bg(Color::DarkGray))
        }

        f.render_stateful_widget(list, layout[PRESET_FIELD_IDX], &mut state_store);

        let icon_block = Block::default()
            .borders(Borders::TOP)
            .border_style(if state.current_input == ICON_FIELD_IDX {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            })
            .title(labels[ICON_FIELD_IDX].clone());
        state.inputs[ICON_FIELD_IDX].set_block(icon_block);
        state.inputs[ICON_FIELD_IDX].set_cursor_line_style(Style::default());
        f.render_widget(&state.inputs[ICON_FIELD_IDX], layout[ICON_FIELD_IDX]);

        let filename_block = Block::default()
            .borders(Borders::TOP)
            .border_style(if state.current_input == FILENAME_FIELD_IDX {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(labels[FILENAME_FIELD_IDX].clone())
            .title(
                Line::from(crate::utils::filename_from_preset(
                    Some(state.inputs[FILENAME_FIELD_IDX].lines().join("")),
                    state.preset.clone(),
                ))
                .alignment(Alignment::Right),
            );
        state.inputs[FILENAME_FIELD_IDX].set_block(filename_block);
        state.inputs[FILENAME_FIELD_IDX].set_cursor_line_style(Style::default());
        f.render_widget(
            &state.inputs[FILENAME_FIELD_IDX],
            layout[FILENAME_FIELD_IDX],
        );

        let name_value = state.inputs[NAME_FIELD_IDX].lines().join("");
        let name_block = Block::default()
            .borders(Borders::TOP)
            .border_style(if state.current_input == NAME_FIELD_IDX {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .title(format!("{}", labels[NAME_FIELD_IDX]))
            .title(
                Line::from(if name_value.is_empty() {
                    String::from("usage: <Icon{} />")
                } else {
                    format!("usage: <Icon{} />", name_value)
                })
                .alignment(Alignment::Right),
            );
        state.inputs[NAME_FIELD_IDX].set_block(name_block);
        state.inputs[NAME_FIELD_IDX].set_cursor_line_style(Style::default());
        f.render_widget(&state.inputs[NAME_FIELD_IDX], layout[NAME_FIELD_IDX]);

        let footer_text = if let Some(message) = &state.status_message {
            message.clone()
        } else {
            String::from("Tab to move, Enter on Name to submit, Esc to cancel, Cmd/Ctrl+V to paste")
        };
        let footer_color = if state.status_is_error {
            Color::Red
        } else {
            Color::DarkGray
        };
        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(footer_color));
        f.render_widget(footer, layout[FOOTER_FIELD_IDX]);
    }
}
