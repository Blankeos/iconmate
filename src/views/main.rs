use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    app_state::{App, AppFocus},
    utils::IconEntry,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainStateFocus {
    Normal,
    Search,
}

#[derive(Debug)]
pub struct MainState {
    pub main_state_focus: MainStateFocus,
    pub search_items_value: String,

    pub search_textarea: TextArea<'static>,
}

impl MainState {
    pub fn new() -> Self {
        Self {
            main_state_focus: MainStateFocus::Normal,
            search_items_value: String::new(),
            search_textarea: TextArea::default(),
        }
    }

    pub fn handlekeys_search(&mut self, input: &Input, app: &mut App) {
        match input.key {
            Key::Esc => {
                self.main_state_focus = MainStateFocus::Normal;
                app.app_focus = AppFocus::Main;
                self.search_textarea = TextArea::default();
                self.search_items_value = String::from("");
            }
            Key::Enter => {
                app.app_focus = AppFocus::Main;
                self.main_state_focus = MainStateFocus::Normal;
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
            Key::Char('/') => {
                self.main_state_focus = MainStateFocus::Search;
            }
            Key::Up | Key::Char('k') => {
                let item_count = if !app.filtered_items.is_empty() {
                    app.filtered_items.len()
                } else {
                    app.items.len()
                };
                if app.selected_index > 0 {
                    app.selected_index -= 1;
                } else {
                    app.selected_index = item_count.saturating_sub(1);
                }
            }
            Key::Down | Key::Char('j') => {
                let item_count = if !app.filtered_items.is_empty() {
                    app.filtered_items.len()
                } else {
                    app.items.len()
                };
                if app.selected_index < item_count.saturating_sub(1) {
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
            .collect()
    }
}

impl App {
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

pub fn render_sidebar(f: &mut Frame, area: Rect, _app: &App) {
    let ascii_art = "░▀█▀░█▀▀░█▀█░█▀█░█▄█░█▀█░▀█▀░█▀▀░\n\
         ░░█░░█░░░█░█░█░█░█░█░█▀█░░█░░█▀▀░\n\
         ░▀▀▀░▀▀▀░▀▀▀░▀░▀░▀░▀░▀░▀░░▀░░▀▀▀░";
    let items: Vec<ListItem> = vec![
        ListItem::new("a  - Add"),
        ListItem::new("d  - Delete"),
        ListItem::new("↑↓ - Navigate (or k,j)"),
        ListItem::new("?  - Help"),
        ListItem::new("/  - Search"),
        ListItem::new("q  - Quit"),
    ];
    let list = List::new(items).highlight_symbol("→ ");
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);

    let inner_block = Block::default()
        .title("Defaults")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);
    let inner_list = List::new(vec![
        ListItem::new("Folder"),
        ListItem::new(_app.config.folder.as_str()),
        ListItem::new(""),
        ListItem::new("Preset"),
        ListItem::new(match &_app.config.preset {
            Some(p) => p.as_str(),
            None => "<none>",
        }),
    ]);

    let vertical_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(7),
        ])
        .split(area);

    let ascii_paragraph = Paragraph::new(ascii_art)
        .style(Style::default().fg(Color::Rgb(74, 222, 128)))
        .alignment(Alignment::Center);
    f.render_widget(ascii_paragraph, vertical_layout[0]);
    f.render_widget(list.block(list_block), vertical_layout[1]);
    f.render_widget(inner_list.block(inner_block), vertical_layout[2]);
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
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    if is_searching {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Length(11),
                Constraint::Fill(1),
                Constraint::Min(0),
            ])
            .split(main_chunks[0]);
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
        f.render_widget(search_paragraph, main_chunks[0]);
    }

    let item_list = if app.filtered_items.is_empty() && !main_state.search_items_value.is_empty() {
        &app.filtered_items
    } else if main_state.search_items_value.is_empty() {
        &app.items
    } else {
        &app.filtered_items
    };

    let rows = item_list.iter().map(|item| {
        Row::new(vec![
            Cell::from(item.name.as_str()),
            Cell::from(item.file_path.as_str()),
        ])
    });

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
    state.select(Some(app.selected_index));
    f.render_stateful_widget(table, main_chunks[1], &mut state);
}
