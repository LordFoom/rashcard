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
}
#[derive(Debug)]
pub struct App<'a> {
    pub running: bool,
    pub state: State,
    pub verbosity: u8,
    pub input_area: TextArea<'a>,
}

impl App<'_> {
    pub fn from_arguments(args: &Args) -> Self {
        Self {
            running: true,
            state: State::Idling,
            verbosity: args.verbosity.clone(),
            input_area: TextArea::default(),
        }
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn show_add_flashcard(&mut self) {
        self.set_state(State::AddFlashcard)
    }

    pub fn stop_running(&mut self) {
        self.running = false;
    }

    pub fn show_next_flashcard(&mut self) {
        self.set_state(State::ShowFlashcard);
    }

    pub fn flip_flashcard(&mut self) {
        self.set_state(State::FlipFlashcard);
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
