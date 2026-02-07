use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use nucleo_matcher::{
    Config, Matcher,
    pattern::{CaseMatching, Normalization, Pattern},
};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    app_state::{App, AppEvent, AppFocus, IconifyCollectionListItem, IconifySearchPayload},
    iconify::IconifyClient,
    utils::popup_area,
};

const SEARCH_DEBOUNCE_MS: u64 = 280;
const SEARCH_LIMIT: u32 = 80;

#[derive(Debug, Clone)]
struct FuzzyCandidate<'a> {
    index: usize,
    haystack: Cow<'a, str>,
}

impl AsRef<str> for FuzzyCandidate<'_> {
    fn as_ref(&self) -> &str {
        &self.haystack
    }
}

fn fuzzy_rank_indices(query: &str, candidates: Vec<FuzzyCandidate<'_>>) -> Vec<usize> {
    if query.trim().is_empty() {
        return candidates
            .into_iter()
            .map(|candidate| candidate.index)
            .collect();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut matched = pattern.match_list(candidates, &mut matcher);
    matched.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.index.cmp(&b.0.index)));

    matched
        .into_iter()
        .map(|(candidate, _)| candidate.index)
        .collect()
}

fn fuzzy_filter_collections(
    collections: &[IconifyCollectionListItem],
    query: &str,
) -> Vec<IconifyCollectionListItem> {
    let candidates = collections
        .iter()
        .enumerate()
        .map(|(index, item)| FuzzyCandidate {
            index,
            haystack: Cow::Owned(format!("{} {}", item.prefix, item.name)),
        })
        .collect::<Vec<_>>();

    fuzzy_rank_indices(query, candidates)
        .into_iter()
        .map(|index| collections[index].clone())
        .collect()
}

fn fuzzy_filter_icons(icons: &[String], query: &str) -> Vec<String> {
    let candidates = icons
        .iter()
        .enumerate()
        .map(|(index, icon)| FuzzyCandidate {
            index,
            haystack: Cow::Borrowed(icon.as_str()),
        })
        .collect::<Vec<_>>();

    fuzzy_rank_indices(query, candidates)
        .into_iter()
        .map(|index| icons[index].clone())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconifySearchTab {
    Collections,
    Icons,
}

#[derive(Debug)]
pub struct IconifySearchPopupState {
    pub search_textarea: TextArea<'static>,
    pub search_value: String,
    pub active_tab: IconifySearchTab,

    pub selected_collection_index: usize,
    pub selected_icon_index: usize,

    pub all_collections: Vec<IconifyCollectionListItem>,
    pub filtered_collections: Vec<IconifyCollectionListItem>,
    pub search_icons: Vec<String>,
    pub collection_icons: Vec<String>,
    pub collection_icons_prefix: Option<String>,
    pub visible_icons: Vec<String>,
    pub selected_collection_filter: Option<String>,

    pub pending_search_query: Option<String>,
    pub debounce_deadline: Option<Instant>,

    pub latest_collections_request_id: u64,
    pub latest_search_request_id: u64,
    pub latest_collection_icons_request_id: u64,

    pub is_loading_collections: bool,
    pub is_loading_search: bool,
    pub is_loading_collection_icons: bool,

    pub status_message: Option<String>,
    pub status_is_error: bool,
}

impl IconifySearchPopupState {
    pub fn new() -> Self {
        Self {
            search_textarea: TextArea::default(),
            search_value: String::new(),
            active_tab: IconifySearchTab::Collections,
            selected_collection_index: 0,
            selected_icon_index: 0,
            all_collections: Vec::new(),
            filtered_collections: Vec::new(),
            search_icons: Vec::new(),
            collection_icons: Vec::new(),
            collection_icons_prefix: None,
            visible_icons: Vec::new(),
            selected_collection_filter: None,
            pending_search_query: None,
            debounce_deadline: None,
            latest_collections_request_id: 0,
            latest_search_request_id: 0,
            latest_collection_icons_request_id: 0,
            is_loading_collections: false,
            is_loading_search: false,
            is_loading_collection_icons: false,
            status_message: None,
            status_is_error: false,
        }
    }

    fn active_collections(&self) -> &[IconifyCollectionListItem] {
        if self.search_value.trim().is_empty() {
            &self.all_collections
        } else {
            &self.filtered_collections
        }
    }

    fn refresh_filtered_collections(&mut self) {
        let query = self.search_value.trim();
        if query.is_empty() {
            self.filtered_collections.clear();
            return;
        }

        self.filtered_collections = fuzzy_filter_collections(&self.all_collections, query);
    }

    fn sync_search_dispatch_state(&mut self) {
        self.pending_search_query = None;
        self.debounce_deadline = None;

        let query = self.search_value.trim().to_string();
        if query.is_empty()
            || self.active_tab != IconifySearchTab::Icons
            || self.selected_collection_filter.is_some()
        {
            self.is_loading_search = false;
            return;
        }

        self.pending_search_query = Some(query);
        self.debounce_deadline = Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
        self.is_loading_search = true;
    }

    fn clear_search_input(&mut self) {
        self.search_textarea = TextArea::default();
        self.search_value.clear();
        self.filtered_collections.clear();
        self.search_icons.clear();
        self.selected_icon_index = 0;
        self.sync_search_dispatch_state();
        self.clear_status();
    }

    fn selected_collection_prefix(&self) -> Option<String> {
        self.active_collections()
            .get(self.selected_collection_index)
            .map(|item| item.prefix.clone())
    }

    fn selected_icon_name(&self) -> Option<String> {
        self.visible_icons.get(self.selected_icon_index).cloned()
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some(message);
        self.status_is_error = is_error;
    }

    fn clear_status(&mut self) {
        self.status_message = None;
        self.status_is_error = false;
    }

    fn clamp_collection_selection(&mut self) {
        let len = self.active_collections().len();
        if len == 0 {
            self.selected_collection_index = 0;
        } else if self.selected_collection_index >= len {
            self.selected_collection_index = len.saturating_sub(1);
        }
    }

    fn clamp_icon_selection(&mut self) {
        let len = self.visible_icons.len();
        if len == 0 {
            self.selected_icon_index = 0;
        } else if self.selected_icon_index >= len {
            self.selected_icon_index = len.saturating_sub(1);
        }
    }

    fn update_search_value(&mut self) {
        self.search_value = self.search_textarea.lines().join("");

        self.clear_status();
        self.selected_collection_index = 0;
        self.selected_icon_index = 0;

        self.refresh_filtered_collections();

        let query = self.search_value.trim().to_string();
        if query.is_empty() {
            self.filtered_collections.clear();
            self.search_icons.clear();
            self.refresh_visible_icons();
            self.sync_search_dispatch_state();
            return;
        }

        self.search_icons.clear();
        self.refresh_visible_icons();
        self.sync_search_dispatch_state();
    }

    fn refresh_visible_icons(&mut self) {
        if let Some(prefix) = &self.selected_collection_filter {
            if self.collection_icons_prefix.as_deref() != Some(prefix.as_str()) {
                self.visible_icons.clear();
                self.clamp_icon_selection();
                return;
            }

            let query = self.search_value.trim();
            if query.is_empty() {
                self.visible_icons = self.collection_icons.clone();
            } else {
                self.visible_icons = fuzzy_filter_icons(&self.collection_icons, query);
            }
        } else {
            self.visible_icons = self.search_icons.clone();
        }

        self.clamp_icon_selection();
    }

    fn move_collection_selection(&mut self, delta: i32) {
        let len = self.active_collections().len();
        if len == 0 {
            self.selected_collection_index = 0;
            return;
        }

        let next = (self.selected_collection_index as i32 + delta).rem_euclid(len as i32) as usize;
        self.selected_collection_index = next;
    }

    fn move_icon_selection(&mut self, delta: i32) {
        let len = self.visible_icons.len();
        if len == 0 {
            self.selected_icon_index = 0;
            return;
        }

        let next = (self.selected_icon_index as i32 + delta).rem_euclid(len as i32) as usize;
        self.selected_icon_index = next;
    }
}

enum PopupAction {
    None,
    Close,
    OpenCollection(String),
    FillAddPopup(String),
    OpenIconInBrowser(String),
}

impl App {
    fn next_request_id(&mut self) -> u64 {
        self.next_async_request_id = self.next_async_request_id.saturating_add(1);
        self.next_async_request_id
    }

    pub fn init_iconify_search_popup(&mut self) {
        self.app_focus = AppFocus::IconifySearchPopup;
        self.iconify_search_popup_state = Some(IconifySearchPopupState::new());
        self.request_iconify_collections();
    }

    fn close_iconify_search_popup(&mut self) {
        self.app_focus = AppFocus::Main;
        self.iconify_search_popup_state = None;
    }

    pub fn handlekeys_iconify_search_popup(&mut self, input: Input) {
        let mut action = PopupAction::None;

        if let Some(state) = self.iconify_search_popup_state.as_mut() {
            match input.key {
                Key::Esc => action = PopupAction::Close,
                Key::Tab => {
                    match state.active_tab {
                        IconifySearchTab::Collections => {
                            state.active_tab = IconifySearchTab::Icons;
                        }
                        IconifySearchTab::Icons => {
                            state.selected_collection_filter = None;
                            state.clear_search_input();
                            state.active_tab = IconifySearchTab::Collections;
                            state.is_loading_collection_icons = false;
                            state.refresh_visible_icons();
                        }
                    }
                    state.sync_search_dispatch_state();
                }
                Key::Up => match state.active_tab {
                    IconifySearchTab::Collections => state.move_collection_selection(-1),
                    IconifySearchTab::Icons => state.move_icon_selection(-1),
                },
                Key::Down => match state.active_tab {
                    IconifySearchTab::Collections => state.move_collection_selection(1),
                    IconifySearchTab::Icons => state.move_icon_selection(1),
                },
                Key::Enter => match state.active_tab {
                    IconifySearchTab::Collections => {
                        if let Some(prefix) = state.selected_collection_prefix() {
                            state.clear_search_input();
                            action = PopupAction::OpenCollection(prefix);
                        }
                    }
                    IconifySearchTab::Icons => {
                        if let Some(icon_name) = state.selected_icon_name() {
                            action = PopupAction::FillAddPopup(icon_name);
                        } else {
                            state.set_status("No icon selected.".to_string(), true);
                        }
                    }
                },
                Key::Char('o') if input.ctrl => {
                    if state.active_tab == IconifySearchTab::Icons {
                        if let Some(icon_name) = state.selected_icon_name() {
                            action = PopupAction::OpenIconInBrowser(icon_name);
                        } else {
                            state.set_status("No icon selected.".to_string(), true);
                        }
                    }
                }
                _ => {
                    state.search_textarea.input(input);
                    state.update_search_value();
                }
            }
        }

        match action {
            PopupAction::None => {}
            PopupAction::Close => self.close_iconify_search_popup(),
            PopupAction::OpenCollection(prefix) => self.open_collection_icons(prefix),
            PopupAction::FillAddPopup(icon_name) => {
                self.close_iconify_search_popup();
                self.init_add_popup_with_icon_source(&icon_name);
            }
            PopupAction::OpenIconInBrowser(icon_name) => {
                self.open_icon_browser_preview(icon_name);
            }
        }
    }

    pub fn tick_iconify_search_popup(&mut self) {
        let query_to_dispatch = self.iconify_search_popup_state.as_ref().and_then(|state| {
            if state.active_tab != IconifySearchTab::Icons
                || state.selected_collection_filter.is_some()
            {
                return None;
            }

            let deadline = state.debounce_deadline?;
            if Instant::now() >= deadline {
                state.pending_search_query.clone()
            } else {
                None
            }
        });

        if let Some(query) = query_to_dispatch {
            self.dispatch_iconify_search(query);
        }
    }

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::IconifyCollectionsLoaded { request_id, result } => {
                if let Some(state) = self.iconify_search_popup_state.as_mut() {
                    if request_id != state.latest_collections_request_id {
                        return;
                    }

                    state.is_loading_collections = false;

                    match result {
                        Ok(items) => {
                            state.all_collections = items;
                            state.refresh_filtered_collections();
                            state.clamp_collection_selection();
                            if state.all_collections.is_empty() {
                                state.set_status("No Iconify collections found.".to_string(), true);
                            } else if !state.is_loading_search {
                                state.clear_status();
                            }
                        }
                        Err(error) => state.set_status(error, true),
                    }
                }
            }
            AppEvent::IconifySearchLoaded {
                request_id,
                query,
                result,
            } => {
                if let Some(state) = self.iconify_search_popup_state.as_mut() {
                    if request_id != state.latest_search_request_id {
                        return;
                    }

                    if query != state.search_value.trim() {
                        return;
                    }

                    if state.active_tab != IconifySearchTab::Icons
                        || state.selected_collection_filter.is_some()
                    {
                        state.is_loading_search = false;
                        return;
                    }

                    state.is_loading_search = false;

                    match result {
                        Ok(payload) => {
                            state.search_icons = payload.icons;
                            state.clamp_collection_selection();
                            state.refresh_visible_icons();

                            if state.search_icons.is_empty()
                                && state.active_collections().is_empty()
                            {
                                state.set_status(
                                    "No matching icons or collections.".to_string(),
                                    false,
                                );
                            } else {
                                state.clear_status();
                            }
                        }
                        Err(error) => {
                            state.search_icons.clear();
                            state.refresh_visible_icons();
                            state.set_status(error, true);
                        }
                    }
                }
            }
            AppEvent::IconifyCollectionIconsLoaded {
                request_id,
                prefix,
                result,
            } => {
                if let Some(state) = self.iconify_search_popup_state.as_mut() {
                    if request_id != state.latest_collection_icons_request_id {
                        return;
                    }

                    state.is_loading_collection_icons = false;

                    match result {
                        Ok(icons) => {
                            state.collection_icons_prefix = Some(prefix);
                            state.collection_icons = icons;
                            state.refresh_visible_icons();

                            if state.collection_icons.is_empty() {
                                state.set_status("No icons in this collection.".to_string(), false);
                            } else if !state.search_value.trim().is_empty()
                                && state.visible_icons.is_empty()
                            {
                                state.set_status(
                                    "No matching icons in this collection.".to_string(),
                                    false,
                                );
                            } else {
                                state.clear_status();
                            }
                        }
                        Err(error) => {
                            state.collection_icons.clear();
                            state.collection_icons_prefix = None;
                            state.refresh_visible_icons();
                            state.set_status(error, true);
                        }
                    }
                }
            }
        }
    }

    fn request_iconify_collections(&mut self) {
        let request_id = self.next_request_id();

        let Some(state) = self.iconify_search_popup_state.as_mut() else {
            return;
        };

        state.latest_collections_request_id = request_id;
        state.is_loading_collections = true;
        state.set_status("Loading collections...".to_string(), false);

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = async {
                let client = IconifyClient::from_env().map_err(|error| error.to_string())?;
                let response = client
                    .collections()
                    .await
                    .map_err(|error| error.to_string())?;

                let mut collections: Vec<IconifyCollectionListItem> = response
                    .collections
                    .into_iter()
                    .map(|(prefix, meta)| IconifyCollectionListItem {
                        name: meta.display_name(&prefix),
                        total: meta.total,
                        prefix,
                    })
                    .collect();

                collections.sort_by(|a, b| a.prefix.cmp(&b.prefix));
                Ok::<Vec<IconifyCollectionListItem>, String>(collections)
            }
            .await;

            let _ = tx.send(AppEvent::IconifyCollectionsLoaded { request_id, result });
        });
    }

    fn dispatch_iconify_search(&mut self, query: String) {
        let request_id = self.next_request_id();

        let Some(state) = self.iconify_search_popup_state.as_mut() else {
            return;
        };

        if state.active_tab != IconifySearchTab::Icons || state.selected_collection_filter.is_some()
        {
            state.pending_search_query = None;
            state.debounce_deadline = None;
            state.is_loading_search = false;
            return;
        }

        state.pending_search_query = None;
        state.debounce_deadline = None;
        state.latest_search_request_id = request_id;
        state.is_loading_search = true;

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = async {
                let client = IconifyClient::from_env().map_err(|error| error.to_string())?;
                let response = client
                    .search(&query, Some(SEARCH_LIMIT), None, false)
                    .await
                    .map_err(|error| error.to_string())?;

                Ok::<IconifySearchPayload, String>(IconifySearchPayload {
                    icons: response.icons,
                })
            }
            .await;

            let _ = tx.send(AppEvent::IconifySearchLoaded {
                request_id,
                query,
                result,
            });
        });
    }

    fn open_collection_icons(&mut self, prefix: String) {
        let mut should_fetch = false;

        if let Some(state) = self.iconify_search_popup_state.as_mut() {
            state.active_tab = IconifySearchTab::Icons;
            state.selected_collection_filter = Some(prefix.clone());
            state.selected_icon_index = 0;
            state.refresh_visible_icons();
            state.sync_search_dispatch_state();

            should_fetch = state.collection_icons_prefix.as_deref() != Some(prefix.as_str());
            if should_fetch {
                state.collection_icons.clear();
                state.refresh_visible_icons();
            }
        }

        if !should_fetch {
            return;
        }

        let request_id = self.next_request_id();

        let Some(state) = self.iconify_search_popup_state.as_mut() else {
            return;
        };

        state.latest_collection_icons_request_id = request_id;
        state.is_loading_collection_icons = true;
        state.set_status(format!("Loading icons for collection '{prefix}'..."), false);

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = async {
                let client = IconifyClient::from_env().map_err(|error| error.to_string())?;
                let response = client
                    .collection(&prefix)
                    .await
                    .map_err(|error| error.to_string())?;

                let icons = response
                    .icons
                    .into_iter()
                    .map(|icon| format!("{}:{icon}", response.prefix))
                    .collect::<Vec<_>>();

                Ok::<Vec<String>, String>(icons)
            }
            .await;

            let _ = tx.send(AppEvent::IconifyCollectionIconsLoaded {
                request_id,
                prefix,
                result,
            });
        });
    }

    fn open_icon_browser_preview(&mut self, icon_name: String) {
        let Some(url) = icones_collection_url(&icon_name) else {
            if let Some(state) = self.iconify_search_popup_state.as_mut() {
                state.set_status(
                    format!("Cannot open Icones page for invalid icon name '{icon_name}'."),
                    true,
                );
            }
            return;
        };

        if let Some(state) = self.iconify_search_popup_state.as_mut() {
            state.set_status("Opening icon in browser...".to_string(), false);
        }

        match crate::viewer::open_url_in_browser(&url) {
            Ok(()) => {
                if let Some(state) = self.iconify_search_popup_state.as_mut() {
                    state.set_status(format!("Opened Icones page: {url}"), false);
                }
            }
            Err(error) => {
                if let Some(state) = self.iconify_search_popup_state.as_mut() {
                    state.set_status(format!("Failed to open browser: {error}"), true);
                }
            }
        }
    }
}

fn icones_collection_url(icon_name: &str) -> Option<String> {
    let (prefix, _) = icon_name.split_once(':')?;
    Some(format!(
        "https://icones.js.org/collection/{prefix}?icon={icon_name}"
    ))
}

pub fn render_iconify_search_popup(f: &mut Frame, app: &mut App) {
    let area = popup_area(f.area(), 90, 24);
    f.render_widget(ratatui::widgets::Clear, area);

    let title = Block::bordered()
        .title("Iconify Search")
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title_style(Style::default().fg(Color::White))
        .title_alignment(Alignment::Center);
    f.render_widget(title, area);

    let inner = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let Some(state) = app.iconify_search_popup_state.as_mut() else {
        return;
    };

    let search_block = Block::default().borders(Borders::TOP).title("Search");
    state.search_textarea.set_block(search_block);
    state
        .search_textarea
        .set_cursor_line_style(Style::default());
    f.render_widget(&state.search_textarea, inner[0]);

    let tabs_label = match state.active_tab {
        IconifySearchTab::Collections => "[Collections]    Icons",
        IconifySearchTab::Icons => "Collections    [Icons]",
    };
    let tabs = Paragraph::new(tabs_label)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);
    f.render_widget(tabs, inner[1]);

    match state.active_tab {
        IconifySearchTab::Collections => {
            let collection_items = state.active_collections();
            let items: Vec<ListItem> = if collection_items.is_empty() {
                vec![ListItem::new("No collections")]
            } else {
                collection_items
                    .iter()
                    .map(|item| {
                        let label = match item.total {
                            Some(total) => format!("{} ({}) - {}", item.prefix, total, item.name),
                            None => format!("{} - {}", item.prefix, item.name),
                        };
                        ListItem::new(label)
                    })
                    .collect()
            };

            let mut list_state = ratatui::widgets::ListState::default();
            if !collection_items.is_empty() {
                list_state.select(Some(state.selected_collection_index));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::TOP).title("Collections"))
                .highlight_symbol("> ")
                .highlight_style(Style::default().bg(Color::DarkGray));
            f.render_stateful_widget(list, inner[2], &mut list_state);
        }
        IconifySearchTab::Icons => {
            let items: Vec<ListItem> = if state.visible_icons.is_empty() {
                vec![ListItem::new("No icons")]
            } else {
                state
                    .visible_icons
                    .iter()
                    .map(|icon| ListItem::new(icon.clone()))
                    .collect()
            };

            let mut list_state = ratatui::widgets::ListState::default();
            if !state.visible_icons.is_empty() {
                list_state.select(Some(state.selected_icon_index));
            }

            let title = if let Some(filter_prefix) = &state.selected_collection_filter {
                format!("Icons (collection: {filter_prefix})")
            } else {
                "Icons".to_string()
            };

            let list = List::new(items)
                .block(Block::default().borders(Borders::TOP).title(title))
                .highlight_symbol("> ")
                .highlight_style(Style::default().bg(Color::DarkGray));
            f.render_stateful_widget(list, inner[2], &mut list_state);
        }
    }

    let loading_message = if state.is_loading_collection_icons {
        Some("Loading collection icons...")
    } else if state.is_loading_search {
        Some("Searching Iconify...")
    } else if state.is_loading_collections {
        Some("Loading collections...")
    } else {
        None
    };

    let status_message = loading_message
        .map(std::string::ToString::to_string)
        .or_else(|| state.status_message.clone())
        .unwrap_or_default();
    let status_color = if state.status_is_error {
        Color::Red
    } else {
        Color::DarkGray
    };
    let status = Paragraph::new(status_message)
        .alignment(Alignment::Left)
        .style(Style::default().fg(status_color));
    f.render_widget(status, inner[3]);

    let help_text = if state.active_tab == IconifySearchTab::Collections {
        "Tab switch tabs | Enter view icons | Up/Down move | Esc close"
    } else {
        "Enter autofill Add popup | Ctrl+o open in browser | Up/Down move | Tab switch | Esc close"
    };
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, inner[4]);
}

#[cfg(test)]
mod tests {
    use super::{
        IconifySearchPopupState, IconifySearchTab, fuzzy_filter_collections, fuzzy_filter_icons,
        icones_collection_url,
    };
    use crate::app_state::{App, AppConfig, AppFocus, IconifyCollectionListItem};
    use tempfile::TempDir;
    use tui_textarea::{Input, Key};

    fn test_app() -> App {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let folder = temp_dir.path().join("icons");

        let config = AppConfig {
            folder: folder.to_string_lossy().into_owned(),
            preset: "normal".to_string(),
            template: None,
            svg_viewer_cmd: None,
            svg_viewer_cmd_source: "test".to_string(),
            global_config_loaded: false,
            project_config_loaded: false,
        };

        App::new(config)
    }

    #[test]
    fn builds_icones_collection_url_for_iconify_name() {
        assert_eq!(
            icones_collection_url("lucide:bean"),
            Some("https://icones.js.org/collection/lucide?icon=lucide:bean".to_string())
        );
    }

    #[test]
    fn returns_none_for_invalid_icon_name() {
        assert_eq!(icones_collection_url("bean"), None);
    }

    #[test]
    fn fuzzy_collections_support_non_substring_queries() {
        let collections = vec![
            IconifyCollectionListItem {
                prefix: "lucide".to_string(),
                name: "Lucide Icons".to_string(),
                total: Some(100),
            },
            IconifyCollectionListItem {
                prefix: "mdi".to_string(),
                name: "Material Design Icons".to_string(),
                total: Some(100),
            },
        ];

        let filtered = fuzzy_filter_collections(&collections, "lcd");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].prefix, "lucide");
    }

    #[test]
    fn fuzzy_icon_filter_supports_non_substring_queries() {
        let icons = vec![
            "lucide:bean".to_string(),
            "lucide:beaker".to_string(),
            "lucide:home".to_string(),
        ];

        let filtered = fuzzy_filter_icons(&icons, "bn");
        assert_eq!(filtered, vec!["lucide:bean".to_string()]);
    }

    #[test]
    fn collection_icon_search_is_local_and_does_not_queue_remote_search() {
        let mut app = test_app();
        app.app_focus = AppFocus::IconifySearchPopup;

        let mut state = IconifySearchPopupState::new();
        state.active_tab = IconifySearchTab::Icons;
        state.selected_collection_filter = Some("lucide".to_string());
        state.collection_icons_prefix = Some("lucide".to_string());
        state.collection_icons = vec![
            "lucide:bean".to_string(),
            "lucide:beaker".to_string(),
            "lucide:home".to_string(),
        ];
        state.refresh_visible_icons();
        app.iconify_search_popup_state = Some(state);

        app.handlekeys_iconify_search_popup(Input {
            key: Key::Char('b'),
            ..Default::default()
        });
        app.handlekeys_iconify_search_popup(Input {
            key: Key::Char('n'),
            ..Default::default()
        });

        let state = app
            .iconify_search_popup_state
            .as_ref()
            .expect("iconify popup state should exist");
        assert_eq!(state.search_value, "bn");
        assert!(state.pending_search_query.is_none());
        assert!(!state.is_loading_search);
        assert_eq!(state.visible_icons, vec!["lucide:bean".to_string()]);
    }

    #[test]
    fn global_icon_search_keeps_remote_query_flow() {
        let mut app = test_app();
        app.app_focus = AppFocus::IconifySearchPopup;

        let mut state = IconifySearchPopupState::new();
        state.active_tab = IconifySearchTab::Icons;
        app.iconify_search_popup_state = Some(state);

        app.handlekeys_iconify_search_popup(Input {
            key: Key::Char('h'),
            ..Default::default()
        });

        let state = app
            .iconify_search_popup_state
            .as_ref()
            .expect("iconify popup state should exist");
        assert_eq!(state.pending_search_query.as_deref(), Some("h"));
        assert!(state.is_loading_search);
    }

    #[test]
    fn j_and_k_type_into_search_input() {
        let mut app = test_app();
        app.app_focus = AppFocus::IconifySearchPopup;
        app.iconify_search_popup_state = Some(IconifySearchPopupState::new());

        app.handlekeys_iconify_search_popup(Input {
            key: Key::Char('j'),
            ..Default::default()
        });
        app.handlekeys_iconify_search_popup(Input {
            key: Key::Char('k'),
            ..Default::default()
        });

        let state = app
            .iconify_search_popup_state
            .as_ref()
            .expect("iconify popup state should exist");
        assert_eq!(state.search_value, "jk");
    }
}
