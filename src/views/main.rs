use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_textarea::{Input, Key, TextArea};

use crate::app_state::{App, AppFocus};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainStateFocus {
    Normal,
    Search,
}

#[derive(Debug)]
pub struct MainState {
    pub main_state_focus: MainStateFocus,
    pub search_items_value: String,
    pub status_message: Option<String>,
    pub status_is_error: bool,

    pub search_textarea: TextArea<'static>,
}

impl MainState {
    pub fn new() -> Self {
        Self {
            main_state_focus: MainStateFocus::Normal,
            search_items_value: String::new(),
            status_message: None,
            status_is_error: false,
            search_textarea: TextArea::default(),
        }
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some(message);
        self.status_is_error = is_error;
    }

    fn clear_status(&mut self) {
        self.status_message = None;
        self.status_is_error = false;
    }

    pub fn handlekeys_search(&mut self, input: &Input, app: &mut App) {
        match input.key {
            Key::Esc => {
                self.main_state_focus = MainStateFocus::Normal;
                app.app_focus = AppFocus::Main;
                self.search_textarea = TextArea::default();
                self.search_items_value = String::from("");
                self.update_filtered_items(app);
            }
            Key::Enter => {
                app.app_focus = AppFocus::Main;
                self.main_state_focus = MainStateFocus::Normal;
            }
            Key::Up => {
                let item_count = if self.search_items_value.is_empty() {
                    app.items.len()
                } else {
                    app.filtered_items.len()
                };
                if item_count == 0 {
                    app.selected_index = 0;
                } else if app.selected_index > 0 {
                    app.selected_index -= 1;
                } else {
                    app.selected_index = item_count - 1;
                }
            }
            Key::Down => {
                let item_count = if self.search_items_value.is_empty() {
                    app.items.len()
                } else {
                    app.filtered_items.len()
                };
                if item_count == 0 {
                    app.selected_index = 0;
                } else if app.selected_index < item_count.saturating_sub(1) {
                    app.selected_index += 1;
                } else {
                    app.selected_index = 0;
                }
            }
            _ => {
                self.search_textarea.input(input.clone());
                self.search_items_value = self.search_textarea.lines().join("");
                self.update_filtered_items(app);
            }
        }
    }

    fn handlekeys_normal(&mut self, input: &Input, app: &mut App) {
        match input.key {
            Key::Char('q') => app.should_quit = true,
            Key::Char('a') => {
                app.init_add_popup();
            }
            Key::Char('d') => {
                app.init_delete_popup();
            }
            Key::Char('r') => {
                app.init_rename_popup();
            }
            Key::Char('o') => match app.open_selected_icon() {
                Ok(crate::viewer::OpenSvgOutcome::OpenedWithCustomCommand) => self.clear_status(),
                Ok(crate::viewer::OpenSvgOutcome::OpenedWithOsDefault) => self.clear_status(),
                Ok(crate::viewer::OpenSvgOutcome::OpenedWithOsDefaultAfterCustomFailure) => self
                    .set_status(
                        "svg_viewer_cmd failed; opened icon via OS default viewer".to_string(),
                        false,
                    ),
                Ok(crate::viewer::OpenSvgOutcome::OpenedWithWebPreview(url)) => self.set_status(
                    format!("Local open failed; opened web preview: {url}"),
                    false,
                ),
                Err(error) => self.set_status(format!("Failed to open icon: {}", error), true),
            },
            Key::Char('/') => {
                self.main_state_focus = MainStateFocus::Search;
            }
            Key::Char('?') => {
                app.init_help_popup();
            }
            Key::Up | Key::Char('k') => {
                let item_count = if self.search_items_value.is_empty() {
                    app.items.len()
                } else {
                    app.filtered_items.len()
                };
                if item_count == 0 {
                    app.selected_index = 0;
                } else if app.selected_index > 0 {
                    app.selected_index -= 1;
                } else {
                    app.selected_index = item_count - 1;
                }
            }
            Key::Down | Key::Char('j') => {
                let item_count = if self.search_items_value.is_empty() {
                    app.items.len()
                } else {
                    app.filtered_items.len()
                };
                if item_count == 0 {
                    app.selected_index = 0;
                } else if app.selected_index < item_count.saturating_sub(1) {
                    app.selected_index += 1;
                } else {
                    app.selected_index = 0;
                }
            }
            _ => {}
        }
    }
    pub fn update_filtered_items(&mut self, app: &mut App) {
        let filter = self.search_items_value.to_lowercase();
        app.filtered_items = app
            .items
            .iter()
            .filter(|entry| {
                let case1 = entry.name.to_lowercase().contains(&filter);
                let case2 = entry.file_path.contains(&filter);

                case1 || case2
            })
            .cloned()
            .collect();
        if app.filtered_items.is_empty() {
            app.selected_index = 0;
        } else if app.selected_index >= app.filtered_items.len() {
            app.selected_index = app.filtered_items.len().saturating_sub(1);
        }
    }
}

impl App {
    pub fn open_selected_icon(&self) -> anyhow::Result<crate::viewer::OpenSvgOutcome> {
        use std::path::Path;

        let item_list = if self.main_state.search_items_value.is_empty() {
            &self.items
        } else {
            &self.filtered_items
        };
        let item = item_list
            .get(self.selected_index)
            .ok_or_else(|| anyhow::anyhow!("No icon selected."))?;

        let file_path = Path::new(&item.file_path);
        let absolute_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            Path::new(&self.config.folder).join(file_path)
        };

        crate::viewer::open_svg_with_fallback(&absolute_path, self.config.svg_viewer_cmd.as_deref())
    }

    pub fn handlekeys_main(&mut self, input: Input) {
        let main_state_ptr = &mut self.main_state as *mut MainState; // Replace MainState with your actual type
        match self.main_state.main_state_focus {
            MainStateFocus::Search => {
                unsafe { (*main_state_ptr).handlekeys_search(&input, self) };
            }
            MainStateFocus::Normal => {
                unsafe { (*main_state_ptr).handlekeys_normal(&input, self) };
            }
        }
    }
}

pub fn render_main_view(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::{Cell, Row, Table};

    let main_state = &app.main_state;
    let is_searching = main_state.main_state_focus == MainStateFocus::Search;

    let header_cells = ["Name", "File"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::White))
        .height(1);

    let main_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let ascii_art = "░▀█▀░█▀▀░█▀█░█▀█░█▄█░█▀█░▀█▀░█▀▀░\n\
         ░░█░░█░░░█░█░█░█░█░█░█▀█░░█░░█▀▀░\n\
         ░▀▀▀░▀▀▀░▀▀▀░▀░▀░▀░▀░▀░▀░░▀░░▀▀▀░";
    let ascii_paragraph = Paragraph::new(ascii_art)
        .style(Style::default().fg(Color::Rgb(74, 222, 128)))
        .alignment(Alignment::Center);
    f.render_widget(ascii_paragraph, main_chunks[0]);

    let tagline = "Add svg icons to your js apps without any dependencies";
    let tagline_paragraph = Paragraph::new(tagline)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(tagline_paragraph, main_chunks[1]);

    let status_text = main_state.status_message.as_deref().unwrap_or("");
    let status_color = if main_state.status_is_error {
        Color::Red
    } else {
        Color::DarkGray
    };
    let status_paragraph = Paragraph::new(status_text)
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Center);
    f.render_widget(status_paragraph, main_chunks[2]);

    if is_searching {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Length(11),
                Constraint::Fill(1),
                Constraint::Min(0),
            ])
            .split(main_chunks[3]);
        let search_label = Paragraph::new("🔍 Search:")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);
        let search_enter_paragraph = Paragraph::new("[Enter]")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Right);
        f.render_widget(search_label, chunks[0]);
        f.render_widget(&main_state.search_textarea, chunks[1]);
        f.render_widget(search_enter_paragraph, chunks[2]);
    } else {
        let search_display = if main_state.search_items_value.is_empty() {
            String::new()
        } else {
            format!("🔍 {}", main_state.search_items_value)
        };
        let search_paragraph = Paragraph::new(search_display.as_str())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);
        f.render_widget(search_paragraph, main_chunks[3]);
    }

    let item_list = if main_state.search_items_value.is_empty() {
        &app.items
    } else {
        &app.filtered_items
    };
    let show_no_results = !main_state.search_items_value.is_empty() && item_list.is_empty();
    let rows: Vec<Row> = if show_no_results {
        vec![Row::new(vec![
            Cell::from("No results").style(Style::default().fg(Color::DarkGray)),
            Cell::from(""),
        ])]
    } else {
        item_list
            .iter()
            .map(|item| {
                Row::new(vec![
                    Cell::from(item.name.as_str()),
                    Cell::from(item.file_path.as_str()),
                ])
            })
            .collect()
    };

    let has_rows = !rows.is_empty();
    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded),
    )
    .column_spacing(2)
    .highlight_symbol("→  ")
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = ratatui::widgets::TableState::default();
    if !show_no_results && has_rows {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(table, main_chunks[4], &mut state);

    let instructions =
        "a Add | d Delete | r Rename | o Open | / Search | ? Help | q Quit | Up/Down (j/k)";
    let version_label = format!("iconmate v{}", env!("CARGO_PKG_VERSION"));
    let footer_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(version_label.chars().count() as u16 + 1),
        ])
        .split(main_chunks[5]);
    let instructions_paragraph = Paragraph::new(instructions)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    let version_paragraph = Paragraph::new(version_label)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Right);
    f.render_widget(instructions_paragraph, footer_layout[0]);
    f.render_widget(version_paragraph, footer_layout[1]);
}
