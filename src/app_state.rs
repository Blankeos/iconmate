use crate::utils::IconEntry;
use crossterm::event::KeyEvent;
use std::sync::mpsc::Receiver;
use tui_textarea::Input;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppFocus {
    Main,
    AddPopup,
    DeletePopup,
    HelpPopup,
    Search,
}

pub struct App {
    pub rx: Receiver<()>,

    pub should_quit: bool,

    pub selected_index: usize,
    pub items: Vec<IconEntry>,

    pub app_focus: AppFocus,

    pub add_popup_state: Option<crate::views::add_popup::AddPopupState>,
}

impl App {
    pub fn new() -> Self {
        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = Self {
            should_quit: false,
            rx: rx,

            selected_index: 0,
            items: Vec::new(),

            app_focus: AppFocus::Main,
            add_popup_state: None,
        };

        app.init_icons();
        app
    }

    fn init_icons(&mut self) {
        // TODO, read the current project
        self.items = vec![
            IconEntry {
                name: "Heart".to_string(),
                file_path: "material:heart.svg".to_string(),
            },
            IconEntry {
                name: "Star".to_string(),
                file_path: "material:star.svg".to_string(),
            },
            IconEntry {
                name: "Home".to_string(),
                file_path: "material:home.svg".to_string(),
            },
            IconEntry {
                name: "User".to_string(),
                file_path: "material:user.svg".to_string(),
            },
            IconEntry {
                name: "Mail".to_string(),
                file_path: "material:mail.svg".to_string(),
            },
            IconEntry {
                name: "Save".to_string(),
                file_path: "material:save.svg".to_string(),
            },
        ];
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

    fn handlekeys_delete_popup(&mut self, _key: Input) {}

    fn handlekeys_help_popup(&mut self, _key: Input) {}

    fn handlekeys_search(&mut self, _key: Input) {}
}
