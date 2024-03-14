use crate::app::{self, App};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Margin, Style};
use ratatui::style::Modifier;
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use ratatui::Frame;

pub fn render_app(frame: &mut Frame, app: &mut App) {
    let size = frame.size();
    //we make some rows
    let rows = Layout::new()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(size);
    let cols = Layout::new()
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .direction(Direction::Horizontal)
        .split(rows[1]);

    //render the top message
    let top_text = match app.state {
        app::State::AddFlashcard => "Ctrl+s to save, Ctrl+b to go back",
        _ => {
            "Welcome to Rashcard, the Rust Flashcard application
         [N]ext | [R]andom | [P]revious | [A]dd | [Q]uit"
        }
    };

    let msg = Paragraph::new(top_text).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(msg, rows[0]);
    let main_display = cols[0];
    //now we do the main panel

    match app.state {
        app::State::Idling => draw_placeholder(frame, main_display),
        app::State::ShowFlashcard => display_current_flashcard(frame, main_display, app),
        app::State::AddFlashcard => display_add_flashcard(frame, main_display, app),
        app::State::DisplaySavedPopup => {
            // info!("Saved! About to display the same");
            draw_saved_popup(frame).unwrap();
            app.close_popup_if_it_is_time();
        }
    }

    //down at the SIDE-BAR, SIDE-BAR, SIDE-BAR!!
    let side_bar = cols[1];

    let message_text = format!(
        r"Total Cards: {}
This is card #{}
Cards displayed: {}",
        app.total_cards, app.current_flashcard_number, app.cards_displayed
    );
    draw_sidebar(&message_text, frame, side_bar)
}

fn draw_sidebar(txt: &str, frame: &mut Frame, rect: Rect) {
    let content = Paragraph::new(txt).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::LightBlue)),
    );

    frame.render_widget(content, rect);
}

///It's a placeholder
fn draw_placeholder(frame: &mut Frame, rect: Rect) {
    let msg = Paragraph::new("R A S H C A R D __ R A S H O M O N").block(
        Block::default().borders(Borders::ALL).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    );

    frame.render_widget(msg, rect);
}

fn draw_saved_popup(f: &mut Frame) -> anyhow::Result<()> {
    display_popup("Saved", f)
}

fn display_popup(msg: &str, f: &mut Frame) -> anyhow::Result<()> {
    let msg = Paragraph::new(msg).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray)),
    );

    let rect = centered_rect(20, 20, f.size());
    f.render_widget(msg, rect);
    Ok(())
}

///Will display the text area we keep in our app for just this occasion
fn display_add_flashcard(frame: &mut Frame, rect: Rect, app: &mut App) {
    // app.init_input_area();
    frame.render_widget(app.input_area.widget(), rect)
}

fn display_current_flashcard(frame: &mut Frame, rect: Rect, app: &mut App) {
    let text = &app.text_lines();
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let mut scrollbar_state = ScrollbarState::new(text.len()).position(app.vertical_scroll);
    let msg = Paragraph::new(text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .scroll((app.vertical_scroll as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(msg, rect);
    frame.render_stateful_widget(
        scrollbar,
        rect.inner(&Margin {
            //inside the block
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    )
}

///Create a 'centered' rect using percentage
fn centered_rect(h: u16, v: u16, rect: Rect) -> Rect {
    //cut into 3 vertical rows
    let layout = Layout::new()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - v) / 2),
            Constraint::Percentage(v),
            Constraint::Percentage((100 - v) / 2),
        ])
        .split(rect);

    //now we split the middle vertical block into 3 columns
    //and we return the middle column
    Layout::new()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - h) / 2),
            Constraint::Percentage(h),
            Constraint::Percentage((100 - h) / 2),
        ])
        .split(layout[1])[1]
}
