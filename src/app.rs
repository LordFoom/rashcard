use std::time::Instant;

use rand::Rng;
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, ScrollbarState},
};
use tui_textarea::TextArea;

use crate::Args;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Idling,
    ShowFlashcard,
    AddFlashcard,
    DisplaySavedPopup,
    DisplayDeletePopup,
}
#[derive(Clone, Copy, Debug)]
pub enum Select {
    Prev,
    Next,
    Random,
}
#[derive(Debug)]
pub struct App<'a> {
    pub running: bool,
    pub state: State,
    pub prior_state: State,
    pub verbosity: u8,
    pub input_area: TextArea<'a>,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub popup_time: Option<Instant>,
    pub current_flashcard_number: usize,
    pub current_flashcard_id: usize,
    pub current_flash_text: String,
    pub total_cards: usize,
    pub first_shown: bool,
    pub cards_displayed: usize,
    pub draw_mode: FlashCardMode,
    ///we want to send a visually signal eg a copy
    pub visual_flicker: bool,
    /// Record of flashcards that have been displayed
    pub flashcard_number_history: Vec<usize>,
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum FlashCardMode {
    Forward,
    Backward,
    Random,
}

pub struct Timer {
    pub start: Instant,
    pub next_card_cycle: usize,
    pub draw_mode: FlashCardMode,
}

impl App<'_> {
    pub fn from_arguments(args: &Args) -> Self {
        Self {
            running: true,
            state: State::Idling,
            prior_state: State::Idling,
            verbosity: args.verbosity,
            input_area: TextArea::default(),
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            popup_time: None,
            current_flashcard_number: 0,
            current_flashcard_id: 0,
            current_flash_text: String::new(),
            total_cards: 0,
            first_shown: false,
            cards_displayed: 0,
            draw_mode: if let Some(mode) = args.mode.clone() {
                mode
            } else {
                FlashCardMode::Random
            },
            visual_flicker: false,
            flashcard_number_history: vec![],
        }
    }

    ///Sets up the app to show the saved popup
    ///This can and should be generalized
    pub fn display_saved_popup(&mut self) {
        self.set_state(State::DisplaySavedPopup);
        let now = Instant::now();
        self.popup_time = Some(now);
    }

    pub fn display_confirm_delete_popup(&mut self) {
        self.set_state(State::DisplayDeletePopup);
    }

    pub fn close_popup_if_it_is_time(&mut self, time: u128) {
        if let Some(inst) = self.popup_time {
            let time_since = inst.elapsed();
            if time_since.as_millis() > time {
                self.restore_prior_state();
            }
        }
    }

    ///Restores the state before the current one,
    ///while making the current on the prior one
    pub fn restore_prior_state(&mut self) {
        std::mem::swap(&mut self.state, &mut self.prior_state);
        self.popup_time = None
    }

    fn set_state(&mut self, state: State) {
        if self.state != state {
            self.prior_state = self.state;
            self.state = state;
        }
    }

    pub fn show_add_flashcard(&mut self) {
        self.set_state(State::AddFlashcard)
    }

    pub fn stop_running(&mut self) {
        self.running = false;
    }

    pub fn idle(&mut self) {
        self.set_state(State::Idling);
        self.first_shown = false;
    }

    pub fn update_flash_text(&mut self, flash_text: &str) {
        self.current_flash_text = flash_text.to_string();
    }

    ///Reset the scrollbar state based on self.current_flash_text
    pub fn reset_scrollbar_state(&mut self) {
        //reset to the beginning bebe
        self.vertical_scroll = 0;
        self.vertical_scroll_state = self
            .vertical_scroll_state
            // .content_length(self.text_lines().len());
            .content_length(100);
    }

    pub fn store_flash_count_histor(&mut self) {}
    pub fn increment_flash_count(&mut self) {
        self.current_flashcard_number += 1;
        if self.current_flashcard_number == self.total_cards {
            self.current_flashcard_number = 0;
        }
    }

    pub fn decrement_flash_count(&mut self) {
        if self.current_flashcard_number == 0 {
            self.current_flashcard_number = self.total_cards - 1
        } else {
            self.current_flashcard_number -= 1;
        }
    }

    pub fn randomize_flash_count(&mut self) {
        let mut rng = rand::thread_rng();

        loop {
            let tmp = rng.gen_range(0..self.total_cards);
            if tmp != self.current_flashcard_number {
                self.current_flashcard_number = tmp;
                break;
            }
        }
    }

    pub fn show_flash_card(&mut self) {
        self.set_state(State::ShowFlashcard);
    }

    pub fn reset_count(&mut self) {
        self.current_flashcard_number = 0;
    }

    ///Increase count of number of cards which have been displayed on the screen
    pub fn increment_display_count(&mut self) {
        self.cards_displayed += 1;
    }

    pub fn flip_flashcard(&mut self) {
        self.set_state(State::ShowFlashcard);
    }

    ///A vec of lines for the current flashcard
    pub fn text_lines<'a>(&'a self) -> Vec<Line<'a>> {
        let mut full_text = Vec::new();

        for line in self.current_flash_text.split('\n') {
            let ln = Line::from(line);
            full_text.push(ln);
        }
        full_text
    }

    pub fn scroll_down(&mut self) {
        self.vertical_scroll += 1;
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub fn scroll_up(&mut self) {
        if self.vertical_scroll > 0 {
            self.vertical_scroll -= 1;
        }
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub fn has_flashcards(&self) -> bool {
        self.total_cards > 0
    }

    pub fn start_delete(&mut self) {
        self.set_state(State::DisplayDeletePopup);
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
            prior_state: State::Idling,
            state: State::Idling,
            verbosity: 0,
            input_area: init_input_area(),
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            popup_time: None,
            current_flashcard_number: 0,
            current_flashcard_id: 0,
            current_flash_text: String::new(),
            total_cards: 0,
            first_shown: false,
            cards_displayed: 0,
            draw_mode: FlashCardMode::Random,
            visual_flicker: false,
            flashcard_number_history: vec![],
        }
    }
}

mod test {

    #[allow(unused_imports)]
    use super::*;

    #[test]
    pub fn test_text() {
        // let mut app = app::default();
        // let lines = vec![
        //     "this is the first line",
        //     "this is the second line",
        //     "this is the third line",
        // ];
        // app.input_area = textarea::from(lines);
        // let res = app.text();
        // assert_eq!(
        //     "this is the first line\nthis is the second line\nthis is the third line\n",
        //     res
        // );
    }
}
