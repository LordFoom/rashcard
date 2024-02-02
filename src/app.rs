use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders},
};
use tui_textarea::TextArea;

use crate::Args;

#[derive(Debug)]
pub enum State {
    Idling,
    ShowFlashcard,
    FlipFlashcard,
    AddFlashcard,
    DisplaySavedPopup,
}
#[derive(Debug)]
pub struct App<'a> {
    pub running: bool,
    pub state: State,
    pub prior_state: State,
    pub verbosity: u8,
    pub input_area: TextArea<'a>,
}

impl App<'_> {
    pub fn from_arguments(args: &Args) -> Self {
        Self {
            running: true,
            state: State::Idling,
            prior_state: State::Idling,
            verbosity: args.verbosity.clone(),
            input_area: TextArea::default(),
        }
    }

    fn set_state(&mut self, state: State) {
        self.prior_state = self.state;
        self.state = state;
    }

    pub fn show_add_flashcard(&mut self) {
        self.prior_state = self.state;
        self.set_state(State::AddFlashcard)
    }

    pub fn stop_running(&mut self) {
        self.running = false;
    }

    pub fn idle(&mut self) {
        self.prior_state = self.state;
        self.set_state(State::Idling);
    }

    pub fn show_next_flashcard(&mut self) {
        self.prior_state = self.state;
        self.set_state(State::ShowFlashcard);
    }

    pub fn flip_flashcard(&mut self) {
        self.prior_state = self.state;
        self.set_state(State::FlipFlashcard);
    }

    pub fn display_saved_popup(& mut self) {
        self.prior_state = self.state;
        self.set_state(State::)
    }

    ///Return whatever text there is in the text_area,
    ///as a single string with newlines separating the lines.
    pub fn text(&self) -> String {
        let mut full_text = String::new();
        for line in self.input_area.lines() {
            full_text.push_str(line);
            full_text.push('\n');
        }
        full_text
    }
}

pub fn init_input_area<'a>() -> TextArea<'a> {
    let mut ta = TextArea::default();
    ta.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)),
    );
    ta
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            running: true,
            state: State::Idling,
            verbosity: 0,
            input_area: init_input_area(),
        }
    }
}

mod test {
    use super::*;

    #[test]
    pub fn test_text() {
        let mut app = App::default();
        let lines = vec![
            "this is the first line",
            "this is the second line",
            "this is the third line",
        ];
        app.input_area = TextArea::from(lines);
        let res = app.text();
        assert_eq!(
            "this is the first line\nthis is the second line\nthis is the third line\n",
            res
        );
    }
}
