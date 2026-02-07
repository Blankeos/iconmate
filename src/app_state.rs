use crate::{utils::IconEntry, views::main::MainState};
use std::sync::mpsc::{Receiver, Sender};
use tui_textarea::Input;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppFocus {
    Main,
    AddPopup,
    DeletePopup,
    RenamePopup,
    HelpPopup,
    IconifySearchPopup,
}

#[derive(Debug, Clone)]
pub struct IconifyCollectionListItem {
    pub prefix: String,
    pub name: String,
    pub total: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct IconifySearchPayload {
    pub icons: Vec<String>,
}

#[derive(Debug)]
pub enum AppEvent {
    IconifyCollectionsLoaded {
        request_id: u64,
        result: Result<Vec<IconifyCollectionListItem>, String>,
    },
    IconifySearchLoaded {
        request_id: u64,
        query: String,
        result: Result<IconifySearchPayload, String>,
    },
    IconifyCollectionIconsLoaded {
        request_id: u64,
        prefix: String,
        result: Result<Vec<String>, String>,
    },
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub folder: String,
    pub preset: Option<String>,
    pub template: Option<String>,
    pub svg_viewer_cmd: Option<String>,
    pub svg_viewer_cmd_source: String,
    pub global_config_loaded: bool,
    pub project_config_loaded: bool,
}

pub struct App {
    pub config: AppConfig,

    // App state
    pub tx: Sender<AppEvent>,
    pub rx: Receiver<AppEvent>,

    pub should_quit: bool,

    pub selected_index: usize,

    pub items: Vec<IconEntry>,

    pub filtered_items: Vec<IconEntry>,

    pub app_focus: AppFocus,

    // Deeper states
    pub main_state: crate::views::main::MainState,
    pub add_popup_state: Option<crate::views::add_popup::AddPopupState>,
    pub delete_popup_state: Option<crate::views::delete_popup::DeletePopupState>,
    pub rename_popup_state: Option<crate::views::rename_popup::RenamePopupState>,
    pub iconify_search_popup_state:
        Option<crate::views::iconify_search_popup::IconifySearchPopupState>,

    pub next_async_request_id: u64,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = Self {
            config,

            should_quit: false,
            tx,
            rx: rx,

            selected_index: 0,
            filtered_items: Vec::new(),
            items: Vec::new(),

            app_focus: AppFocus::Main,
            add_popup_state: None,
            delete_popup_state: None,
            rename_popup_state: None,
            iconify_search_popup_state: None,
            next_async_request_id: 0,
            main_state: MainState::new(),
        };

        app.init_icons();
        app
    }

    pub fn init_icons(&mut self) {
        // Try to read the current project's export file
        self.items = match crate::utils::get_existing_icons(&self.config.folder) {
            Ok(icons) => icons,
            Err(_) => Vec::new(),
        };
        self.filtered_items = self.items.clone();
    }

    pub fn update(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.handle_app_event(event);
        }

        self.tick_iconify_search_popup();
    }

    pub fn handlekeys(&mut self, key: Input) {
        match self.app_focus {
            AppFocus::Main => self.handlekeys_main(key),
            AppFocus::AddPopup => self.handlekeys_add_popup(key),
            AppFocus::DeletePopup => self.handlekeys_delete_popup(key),
            AppFocus::RenamePopup => self.handlekeys_rename_popup(key),
            AppFocus::HelpPopup => self.handlekeys_help_popup(key),
            AppFocus::IconifySearchPopup => self.handlekeys_iconify_search_popup(key),
        }
    }
}
