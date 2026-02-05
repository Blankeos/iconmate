use crate::{utils::IconEntry, views::main::MainState};
use std::sync::mpsc::Receiver;
use tui_textarea::Input;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppFocus {
    Main,
    AddPopup,
    DeletePopup,
    HelpPopup,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub folder: String,
    pub preset: Option<String>,
    pub template: Option<String>,
}

pub struct App {
    pub config: AppConfig,

    // App state
    pub rx: Receiver<()>,

    pub should_quit: bool,

    pub selected_index: usize,

    pub items: Vec<IconEntry>,

    pub filtered_items: Vec<IconEntry>,

    pub app_focus: AppFocus,

    // Deeper states
    pub main_state: crate::views::main::MainState,
    pub add_popup_state: Option<crate::views::add_popup::AddPopupState>,
    pub delete_popup_state: Option<crate::views::delete_popup::DeletePopupState>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = Self {
            config,

            should_quit: false,
            rx: rx,

            selected_index: 0,
            filtered_items: Vec::new(),
            items: Vec::new(),

            app_focus: AppFocus::Main,
            add_popup_state: None,
            delete_popup_state: None,
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

    pub fn update(&mut self) {}

    pub fn handlekeys(&mut self, key: Input) {
        match self.app_focus {
            AppFocus::Main => self.handlekeys_main(key),
            AppFocus::AddPopup => self.handlekeys_add_popup(key),
            AppFocus::DeletePopup => self.handlekeys_delete_popup(key),
            AppFocus::HelpPopup => self.handlekeys_help_popup(key),
        }
    }
}
