use std::path::{Path, PathBuf};

use crate::app_state::{App, AppFocus};
use crate::utils::popup_area;

/// Flutter-preset rename: rename the SVG file on disk and update the path
/// string inside the Dart barrel. Leaves the Dart identifier untouched — the
/// user renames via LSP if they want the symbol changed.
fn perform_flutter_rename(
    folder: &str,
    flutter_barrel_file: Option<&str>,
    flutter_barrel_class: Option<&str>,
    current_file_path: &str,
    new_file_input: &str,
) -> anyhow::Result<()> {
    use std::path::Component;

    let current_rel = current_file_path.trim().to_string();
    if current_rel.is_empty() {
        anyhow::bail!("Current icon path is empty");
    }

    let mut new_rel = new_file_input.trim().to_string();
    if new_rel.is_empty() {
        anyhow::bail!("New filename cannot be empty");
    }

    let new_path_check = Path::new(&new_rel);
    if new_path_check.is_absolute() {
        anyhow::bail!("Please provide a relative filename");
    }
    if new_path_check
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("Parent directory traversals are not allowed");
    }

    // Preserve existing extension if the user typed just a bare name.
    if Path::new(&new_rel).extension().is_none() {
        if let Some(ext) = Path::new(&current_rel)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            new_rel = format!("{}.{}", new_rel, ext);
        }
    }

    if new_rel == current_rel {
        anyhow::bail!("Filename is unchanged");
    }

    let folder_path = Path::new(folder);
    let current_abs = folder_path.join(&current_rel);
    if !current_abs.exists() {
        anyhow::bail!("Icon file not found: {}", current_abs.display());
    }

    let new_abs = folder_path.join(&new_rel);
    if new_abs.exists() {
        anyhow::bail!("Target file already exists: {}", new_abs.display());
    }

    let barrel_path: PathBuf = flutter_barrel_file
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(crate::flutter::DEFAULT_FLUTTER_BARREL_FILE));
    let class = flutter_barrel_class.unwrap_or(crate::flutter::DEFAULT_FLUTTER_BARREL_CLASS);

    if !barrel_path.exists() {
        anyhow::bail!("No barrel file found at {}", barrel_path.display());
    }

    let entries = crate::flutter::read_barrel_entries(&barrel_path)?;
    let current_asset = crate::flutter::asset_path_for(folder, &current_rel);
    let new_asset = crate::flutter::asset_path_for(folder, &new_rel);
    let updated = crate::flutter::rename_entry_path(&entries, &current_asset, &new_asset)?;

    if let Some(parent) = new_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&current_abs, &new_abs)?;

    if let Err(err) = crate::flutter::write_barrel(&barrel_path, class, &updated) {
        // Roll back the rename so state stays consistent.
        let _ = std::fs::rename(&new_abs, &current_abs);
        return Err(err);
    }

    Ok(())
}
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph, Wrap};
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

        filename_input.set_cursor_style(
            Style::default()
                .bg(crate::views::theme::ACCENT)
                .fg(crate::views::theme::BASE_BG),
        );

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

        if self.config.preset == "flutter" {
            perform_flutter_rename(
                &self.config.folder,
                self.config.flutter_barrel_file.as_deref(),
                self.config.flutter_barrel_class.as_deref(),
                &item.file_path,
                &new_filename,
            )
            .map_err(|error| error.to_string())?;
        } else {
            crate::utils::rename_icon_entry(&self.config.folder, &item.file_path, &new_filename)
                .map_err(|error| error.to_string())?;
        }

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
    use ratatui::style::Modifier;

    let area = popup_area(f.area(), 74, 16);
    let body_area = crate::views::theme::render_popup_shell(f, area, "Rename File");

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(body_area);

    if let Some(state) = app.rename_popup_state.as_mut() {
        let status = if let Some(item) = &state.item_to_rename {
            format!("Alias: {}\nCurrent file: {}", item.name, item.file_path)
        } else {
            "No icon selected".to_string()
        };
        let status_paragraph = Paragraph::new(status)
            .alignment(Alignment::Left)
            .style(Style::default().fg(crate::views::theme::MUTED_TEXT));
        f.render_widget(status_paragraph, layout[0]);

        let input_block = Block::default()
            .title("New filename")
            .title_style(
                Style::default()
                    .fg(crate::views::theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(crate::views::theme::TEXT));
        state.filename_input.set_block(input_block);
        state.filename_input.set_cursor_line_style(Style::default());
        f.render_widget(&state.filename_input, layout[2]);

        let tip = Paragraph::new(
            "Renames only the file path export target.\nFor alias rename, use your IDE's rename feature via LSP.",
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(crate::views::theme::SUBTLE_TEXT));
        f.render_widget(tip, layout[4]);

        let footer = if let Some(message) = &state.status_message {
            let color = if state.status_is_error {
                crate::views::theme::ERROR
            } else {
                crate::views::theme::MUTED_TEXT
            };
            Paragraph::new(message.clone())
                .alignment(Alignment::Left)
                .style(Style::default().fg(color))
        } else {
            Paragraph::new(crate::views::theme::shortcut_line(&[
                ("Rename", "enter"),
                ("Cancel", "esc"),
                ("Paste", "cmd/ctrl+v"),
            ]))
            .alignment(Alignment::Left)
        };
        f.render_widget(footer, layout[5]);
    }
}
