use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::{Duration, Instant},
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
    pub is_opening_preview: bool,

    pub status_message: Option<String>,
    pub status_is_error: bool,

    pub preview_cache: HashMap<String, PathBuf>,
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
            is_opening_preview: false,
            status_message: None,
            status_is_error: false,
            preview_cache: HashMap::new(),
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
        let query = self.search_value.trim().to_lowercase();
        if query.is_empty() {
            self.filtered_collections.clear();
            return;
        }

        self.filtered_collections = self
            .all_collections
            .iter()
            .filter(|item| {
                item.prefix.to_lowercase().contains(&query)
                    || item.name.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
    }

    fn sync_search_dispatch_state(&mut self) {
        self.pending_search_query = None;
        self.debounce_deadline = None;

        let query = self.search_value.trim().to_string();
        if query.is_empty() || self.active_tab != IconifySearchTab::Icons {
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
            if !self.search_value.trim().is_empty() {
                let prefix_filter = format!("{prefix}:");
                self.visible_icons = self
                    .search_icons
                    .iter()
                    .filter(|icon| icon.starts_with(&prefix_filter))
                    .cloned()
                    .collect();
            } else if self.collection_icons_prefix.as_deref() == Some(prefix.as_str()) {
                self.visible_icons = self.collection_icons.clone();
            } else {
                self.visible_icons.clear();
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
    OpenIconPreview(String),
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
                Key::Up | Key::Char('k') => match state.active_tab {
                    IconifySearchTab::Collections => state.move_collection_selection(-1),
                    IconifySearchTab::Icons => state.move_icon_selection(-1),
                },
                Key::Down | Key::Char('j') => match state.active_tab {
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
                            action = PopupAction::OpenIconPreview(icon_name);
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
            PopupAction::OpenIconPreview(icon_name) => {
                self.open_icon_preview(icon_name);
            }
        }
    }

    pub fn tick_iconify_search_popup(&mut self) {
        let query_to_dispatch = self.iconify_search_popup_state.as_ref().and_then(|state| {
            if state.active_tab != IconifySearchTab::Icons {
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

                            if state.visible_icons.is_empty() {
                                state.set_status("No icons in this collection.".to_string(), false);
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
            AppEvent::IconifyPreviewOpened {
                icon_name,
                temp_file,
                result,
            } => {
                if let Some(state) = self.iconify_search_popup_state.as_mut() {
                    state.is_opening_preview = false;

                    match result {
                        Ok(outcome) => {
                            if let Some(path) = temp_file {
                                state.preview_cache.insert(icon_name, path);
                            }

                            match outcome {
                                crate::viewer::OpenSvgOutcome::OpenedWithCustomCommand => {
                                    state.set_status("Opened icon preview.".to_string(), false)
                                }
                                crate::viewer::OpenSvgOutcome::OpenedWithOsDefault => {
                                    state.set_status("Opened icon preview.".to_string(), false)
                                }
                                crate::viewer::OpenSvgOutcome::OpenedWithOsDefaultAfterCustomFailure => {
                                    state.set_status(
                                        "svg_viewer_cmd failed; opened preview via OS default"
                                            .to_string(),
                                        false,
                                    )
                                }
                                crate::viewer::OpenSvgOutcome::OpenedWithWebPreview(url) => {
                                    state.set_status(
                                        format!(
                                            "Local open failed; opened Iconify web preview: {url}"
                                        ),
                                        false,
                                    )
                                }
                            }
                        }
                        Err(error) => {
                            state.set_status(format!("Failed to open preview: {error}"), true);
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

            if state.search_value.trim().is_empty() {
                should_fetch = state.collection_icons_prefix.as_deref() != Some(prefix.as_str());
                if should_fetch {
                    state.collection_icons.clear();
                    state.refresh_visible_icons();
                }
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

    fn open_icon_preview(&mut self, icon_name: String) {
        let cached_path = self
            .iconify_search_popup_state
            .as_ref()
            .and_then(|state| state.preview_cache.get(&icon_name).cloned())
            .filter(|path| path.exists());

        if let Some(state) = self.iconify_search_popup_state.as_mut() {
            state.is_opening_preview = true;
            if cached_path.is_some() {
                state.set_status(
                    format!("Opening cached preview for '{icon_name}'..."),
                    false,
                );
            } else {
                state.set_status(format!("Downloading preview for '{icon_name}'..."), false);
            }
        }

        let tx = self.tx.clone();
        let viewer_cmd = self.config.svg_viewer_cmd.clone();

        tokio::spawn(async move {
            let operation = async {
                if let Some(path) = cached_path {
                    let outcome =
                        crate::viewer::open_svg_with_fallback(&path, viewer_cmd.as_deref())
                            .map_err(|error| error.to_string())?;
                    return Ok::<(crate::viewer::OpenSvgOutcome, Option<PathBuf>), String>((
                        outcome, None,
                    ));
                }

                let client = IconifyClient::from_env().map_err(|error| error.to_string())?;
                let svg = client
                    .svg(&icon_name)
                    .await
                    .map_err(|error| error.to_string())?;
                let path = icon_preview_temp_path(&icon_name);

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::write(&path, svg).map_err(|error| error.to_string())?;

                let outcome = crate::viewer::open_svg_with_fallback(&path, viewer_cmd.as_deref())
                    .map_err(|error| error.to_string())?;

                Ok((outcome, Some(path)))
            }
            .await;

            match operation {
                Ok((outcome, temp_file)) => {
                    let _ = tx.send(AppEvent::IconifyPreviewOpened {
                        icon_name,
                        temp_file,
                        result: Ok(outcome),
                    });
                }
                Err(error) => {
                    let _ = tx.send(AppEvent::IconifyPreviewOpened {
                        icon_name,
                        temp_file: None,
                        result: Err(error),
                    });
                }
            }
        });
    }
}

fn icon_preview_temp_path(icon_name: &str) -> PathBuf {
    let safe_stem: String = icon_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    icon_name.hash(&mut hasher);
    let suffix = hasher.finish();

    std::env::temp_dir()
        .join("iconmate-preview")
        .join(format!("{}-{suffix:x}.svg", safe_stem))
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

    let loading_message = if state.is_opening_preview {
        Some("Downloading/opening preview...")
    } else if state.is_loading_collection_icons {
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
        "Enter autofill Add popup | Ctrl+o open | Up/Down move | Tab switch | Esc close"
    };
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, inner[4]);
}
