use crossterm::event::KeyCode;
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders},
};
use tui_textarea::TextArea;

#[derive(Debug)]
pub struct FormInput {
    label: String,
    textarea: TextArea<'static>,
    is_focused: bool,
}
impl FormInput {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            textarea: TextArea::default(),
            is_focused: false,
        }
    }

    pub fn with_placeholder(mut self, text: &str) -> Self {
        self.textarea.insert_str(text);
        self
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        use tui_textarea::{Input, Key};

        let input = match key {
            KeyCode::Char(c) => Input {
                key: Key::Char(c),
                ..Default::default()
            },
            KeyCode::Backspace => Input {
                key: Key::Backspace,
                ..Default::default()
            },
            KeyCode::Delete => Input {
                key: Key::Delete,
                ..Default::default()
            },
            KeyCode::Left => Input {
                key: Key::Left,
                ..Default::default()
            },
            KeyCode::Right => Input {
                key: Key::Right,
                ..Default::default()
            },
            KeyCode::Home => Input {
                key: Key::Home,
                ..Default::default()
            },
            KeyCode::End => Input {
                key: Key::End,
                ..Default::default()
            },
            _ => return,
        };

        self.textarea.input(input);
    }

    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
    }

    pub fn widget(&mut self) -> impl ratatui::widgets::Widget + '_ {
        let border_color = if self.is_focused {
            Color::Yellow
        } else {
            Color::Gray
        };

        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(self.label.clone()),
        );

        &self.textarea
    }
}
