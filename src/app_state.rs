use crate::utils::IconEntry;
use crossterm::event::KeyEvent;
use std::sync::mpsc::Receiver;
use tui_textarea::{Input, Key};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppFocus {
    Main,
    AddPopup,
    DeletePopup,
    HelpPopup,
    Search,
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

    // Main State (actually.. could move it there too)
    pub selected_index: usize,
    pub search_items_value: String,
    pub items: Vec<IconEntry>,
    pub filtered_items: Vec<IconEntry>,
    pub app_focus: AppFocus,

    // Deeper states
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
            search_items_value: String::from(""),
            filtered_items: Vec::new(),
            items: Vec::new(),

            app_focus: AppFocus::Main,
            add_popup_state: None,
            delete_popup_state: None,
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
            AppFocus::Search => self.handlekeys_search(key),
        }
    }

    fn handlekeys_help_popup(&mut self, _key: Input) {}

    fn handlekeys_search(&mut self, input: Input) {
        match input.key {
            Key::Esc => {
                self.app_focus = AppFocus::Main;
                self.search_items_value.clear();
            }
            Key::Char(c) => {
                self.search_items_value.push(c);
                self.update_filtered_items();
            }
            Key::Backspace => {
                self.search_items_value.pop();
                self.update_filtered_items();
            }
            Key::Enter => {
                self.app_focus = AppFocus::Main;
            }
            _ => {}
        }
    }

    fn update_filtered_items(&mut self) {
        let filter = self.search_items_value.to_lowercase();
        self.filtered_items = self
            .items
            .iter()
            .filter(|entry| entry.name.to_lowercase().contains(&filter))
            .cloned()
            .collect()
    }
}
