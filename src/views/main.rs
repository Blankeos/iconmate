use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tui_textarea::{Input, Key};

use crate::app_state::{App, AppFocus};
impl App {
    pub fn handlekeys_main(&mut self, input: Input) {
        match self.app_focus {
            AppFocus::Search => {} // Dirty, but uh it's fine
            _ => self.handlekeys_normal(input),
        }
    }

    fn handlekeys_normal(&mut self, input: Input) {
        match input.key {
            Key::Char('q') => self.should_quit = true,
            Key::Char('a') => {
                self.init_add_popup();
            }
            Key::Char('d') => {
                self.init_delete_popup();
            }
            Key::Char('/') => {
                self.app_focus = AppFocus::Search;
            }
            Key::Up | Key::Char('k') => {
                let item_count = if !self.filtered_items.is_empty() {
                    self.filtered_items.len()
                } else {
                    self.items.len()
                };
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                } else {
                    self.selected_index = item_count.saturating_sub(1);
                }
            }
            Key::Down | Key::Char('j') => {
                let item_count = if !self.filtered_items.is_empty() {
                    self.filtered_items.len()
                } else {
                    self.items.len()
                };
                if self.selected_index < item_count.saturating_sub(1) {
                    self.selected_index += 1;
                } else {
                    self.selected_index = 0;
                }
            }
            _ => {}
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
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Max(10),
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

    let search_display = if app.search_items_value.is_empty() && app.app_focus != AppFocus::Search {
        String::new()
    } else {
        let prefix = if app.app_focus == AppFocus::Search {
            "🔍 Search: "
        } else {
            "🔍 "
        };
        format!("{}{}", prefix, app.search_items_value)
    };
    let mut search_style = Style::default().fg(Color::White);
    if app.app_focus == AppFocus::Search {
        search_style = search_style.bg(Color::DarkGray);
    }
    let search_paragraph = Paragraph::new(search_display.as_str())
        .style(search_style)
        .alignment(Alignment::Left);
    f.render_widget(search_paragraph, main_chunks[0]);

    let item_list = if app.filtered_items.is_empty() && !app.search_items_value.is_empty() {
        &app.filtered_items
    } else if app.search_items_value.is_empty() {
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
