use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    widgets::{Block, Paragraph},
    Frame,
};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    app_state::{App, AppFocus},
    utils::IconEntry,
};

#[derive(Debug, Clone)]
struct HomeSearchCandidate {
    index: usize,
    haystack: String,
}

impl AsRef<str> for HomeSearchCandidate {
    fn as_ref(&self) -> &str {
        self.haystack.as_str()
    }
}

fn fuzzy_filter_home_items(items: &[IconEntry], query: &str) -> Vec<IconEntry> {
    let query = query.trim();
    if query.is_empty() {
        return items.to_vec();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);

    let candidates = items
        .iter()
        .enumerate()
        .map(|(index, item)| HomeSearchCandidate {
            index,
            haystack: format!("{} {}", item.name, item.file_path),
        })
        .collect::<Vec<_>>();

    let mut matched = pattern.match_list(candidates, &mut matcher);
    matched.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.index.cmp(&b.0.index)));

    matched
        .into_iter()
        .map(|(candidate, _)| items[candidate.index].clone())
        .collect()
}

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
            Key::Char('i') => {
                app.init_iconify_search_popup();
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
        app.filtered_items = fuzzy_filter_home_items(&app.items, &self.search_items_value);
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

pub fn render_main_view(f: &mut Frame, area: Rect, app: &mut App) {
    use ratatui::{
        style::Modifier,
        widgets::{Cell, Row, Table},
    };

    let main_state = &mut app.main_state;
    let is_searching = main_state.main_state_focus == MainStateFocus::Search;

    let main_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
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
    f.render_widget(
        Paragraph::new(ascii_art)
            .style(Style::default().fg(crate::views::theme::ACCENT))
            .alignment(Alignment::Center),
        main_chunks[0],
    );

    f.render_widget(
        Paragraph::new("Add svg icons to your js apps without any dependencies")
            .style(Style::default().fg(crate::views::theme::SUBTLE_TEXT))
            .alignment(Alignment::Center),
        main_chunks[1],
    );

    let status_text = main_state.status_message.clone().unwrap_or_default();
    let status_color = if main_state.status_is_error {
        crate::views::theme::ERROR
    } else {
        crate::views::theme::SUBTLE_TEXT
    };
    f.render_widget(
        Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .alignment(Alignment::Center),
        main_chunks[2],
    );

    let search_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Length(8),
        ])
        .split(main_chunks[3]);

    f.render_widget(
        Paragraph::new("Search /")
            .style(Style::default().fg(crate::views::theme::MUTED_TEXT))
            .alignment(Alignment::Left),
        search_chunks[0],
    );

    if is_searching {
        main_state.search_textarea.set_block(Block::default());
        main_state.search_textarea.set_cursor_style(
            Style::default()
                .bg(crate::views::theme::ACCENT)
                .fg(crate::views::theme::BASE_BG),
        );
        main_state
            .search_textarea
            .set_cursor_line_style(Style::default());
        f.render_widget(&main_state.search_textarea, search_chunks[1]);
        f.render_widget(
            Paragraph::new("enter")
                .style(Style::default().fg(crate::views::theme::MUTED_TEXT))
                .alignment(Alignment::Right),
            search_chunks[2],
        );
    } else {
        let search_display = if main_state.search_items_value.is_empty() {
            String::new()
        } else {
            main_state.search_items_value.clone()
        };
        let search_color = if main_state.search_items_value.is_empty() {
            crate::views::theme::MUTED_TEXT
        } else {
            crate::views::theme::TEXT
        };
        f.render_widget(
            Paragraph::new(search_display)
                .style(Style::default().fg(search_color))
                .alignment(Alignment::Left),
            search_chunks[1],
        );
    }

    let item_list = if main_state.search_items_value.is_empty() {
        &app.items
    } else {
        &app.filtered_items
    };
    let show_no_results = !main_state.search_items_value.is_empty() && item_list.is_empty();

    let header_cells = ["Name", "File"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(crate::views::theme::MUTED_TEXT)));
    let header = Row::new(header_cells).style(Style::default().fg(crate::views::theme::TEXT));

    let rows: Vec<Row> = if show_no_results {
        vec![Row::new(vec![
            Cell::from("No icons match your search")
                .style(Style::default().fg(crate::views::theme::SUBTLE_TEXT)),
            Cell::from(""),
        ])]
    } else {
        item_list
            .iter()
            .map(|item| {
                Row::new(vec![
                    Cell::from(item.name.as_str())
                        .style(Style::default().fg(crate::views::theme::TEXT)),
                    Cell::from(item.file_path.as_str())
                        .style(Style::default().fg(crate::views::theme::MUTED_TEXT)),
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
    .block(Block::default())
    .column_spacing(2)
    .highlight_symbol("  ")
    .row_highlight_style(
        Style::default()
            .bg(crate::views::theme::ROW_HIGHLIGHT_BG)
            .fg(crate::views::theme::BASE_BG)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ratatui::widgets::TableState::default();
    if !show_no_results && has_rows {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(table, main_chunks[5], &mut state);

    let shortcuts = crate::views::theme::shortcut_line(&[
        ("Add", "a"),
        ("Iconify", "i"),
        ("Delete", "d"),
        ("Rename", "r"),
        ("Open", "o"),
        ("Help", "?"),
        ("Quit", "q"),
    ]);
    let version_label = format!("v{}", env!("CARGO_PKG_VERSION"));
    let footer_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(version_label.chars().count() as u16 + 1),
        ])
        .split(main_chunks[6]);
    f.render_widget(
        Paragraph::new(shortcuts).alignment(Alignment::Left),
        footer_layout[0],
    );
    f.render_widget(
        Paragraph::new(version_label)
            .style(Style::default().fg(crate::views::theme::SUBTLE_TEXT))
            .alignment(Alignment::Right),
        footer_layout[1],
    );
}

#[cfg(test)]
mod tests {
    use super::fuzzy_filter_home_items;
    use crate::utils::IconEntry;

    fn sample_items() -> Vec<IconEntry> {
        vec![
            IconEntry {
                name: "IconMountain".to_string(),
                file_path: "./lucide:mountain.svg".to_string(),
            },
            IconEntry {
                name: "IconHeart".to_string(),
                file_path: "./lucide:heart.svg".to_string(),
            },
            IconEntry {
                name: "IconHouse".to_string(),
                file_path: "./lucide:house.svg".to_string(),
            },
        ]
    }

    #[test]
    fn home_search_matches_fuzzy_name_query() {
        let filtered = fuzzy_filter_home_items(&sample_items(), "ihrt");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "IconHeart");
    }

    #[test]
    fn home_search_matches_fuzzy_file_path_query() {
        let filtered = fuzzy_filter_home_items(&sample_items(), "lchrt");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "IconHeart");
    }

    #[test]
    fn home_search_keeps_all_items_for_empty_query() {
        let items = sample_items();
        let filtered = fuzzy_filter_home_items(&items, "   ");
        assert_eq!(filtered.len(), items.len());
        assert_eq!(filtered[0].name, items[0].name);
        assert_eq!(filtered[1].name, items[1].name);
        assert_eq!(filtered[2].name, items[2].name);
    }
}
