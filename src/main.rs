use anyhow::{Context, Result};
use app::{update_flashcard, State};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use db::default_connection;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use rusqlite::{params, Connection};
use std::time::Duration;
use std::{
    io::{stdout, Stdout},
    thread::sleep,
};
use tracing::{info, instrument, Level};
use tui_textarea::{Input, Key};

use crate::app::App;
mod app;
mod db;

///Command line arguments for clap
#[derive(Parser)]
#[command(
    author = "foom",
    version = "0.1",
    about = "Flashcards in rust",
    long_about = "Flashcard to make knowledge stick like rust to metal"
)]
pub struct Args {
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbosity: u8,
}

fn init_logging(level: u8) {
    //TODO set this via config
    let lvl = match level {
        0 => Level::ERROR, //rock solid confidence
        1 => Level::INFO,  //wibble
        2 => Level::DEBUG, //wobble
        _ => Level::TRACE, //you are the crazy tracer man
    };
    tracing_subscriber::fmt().with_max_level(lvl).init();
}

///TODO Read in flashcards from cli
///TODO read in flashcards from markdown files
fn main() -> Result<()> {
    let args = Args::parse();
    let app = App::from_arguments(&args);
    init_logging(app.verbosity.clone());
    let mut terminal = setup_terminal().context("setup failed")?;
    let conn = default_connection().context("failed to get sql connection")?;
    run(app, &conn, &mut terminal).context("failed running")?;
    // tracing::debug!()
    // let mut terminal = Terminal::new(CrosstermBackend::new(stdout()));
    unsetup_terminal(&mut terminal).context("unsetup failed")
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    // info!("Setting up terminal...");
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("unable to enter alternate screen")?;
    let term = Terminal::new(CrosstermBackend::new(stdout)).context("unable to setup terminal");
    // info!("Terminal setup");
    term
}

fn unsetup_terminal(term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    // info!("Unsetting up terminal...");
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(term.backend_mut(), LeaveAlternateScreen)
        .context("unable to return to main screen")?;
    term.show_cursor().context("unable to show cursor")
}

fn run(
    mut app: App,
    conn: &Connection,
    term: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<()> {
    //create the table if need be
    init_table(&conn)?;
    loop {
        term.draw(|f| render_app(f, &mut app))?;
        read_input(&mut app, conn)?;
        if !app.running {
            // info!("Going down!");
            break;
        }
    }
    Ok(())
}

fn read_input(app: &mut App, conn: &Connection) -> Result<()> {
    if event::poll(Duration::from_millis(250)).context("event poll failed")? {
        match app.state {
            State::AddFlashcard => match event::read().context("event read failed")?.into() {
                Input {
                    key: Key::Char('q'),
                    ctrl: true,
                    ..
                }
                | Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => app.stop_running(),
                Input {
                    key: Key::Char('s'),
                    ctrl: true,
                    ..
                } => save_flashcard(app, conn)?,
                Input {
                    key: Key::Char('j'),
                    ctrl: true,
                    ..
                } => {
                    app.idle();
                    while !app.input_area.is_empty() {
                        app.input_area.move_cursor(tui_textarea::CursorMove::End);
                        app.input_area.delete_line_by_head();
                    }
                }
                input => {
                    app.input_area.input(input);
                    ()
                }
            },
            State::ShowFlashcard | _ => {
                if let Event::Key(key) = event::read().context("event read failed")? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => app.stop_running(),
                        KeyCode::Char('a') | KeyCode::Char('A') => app.show_add_flashcard(),
                        KeyCode::Char('s') | KeyCode::Char('S') => show_next_flashcard(app, conn),
                        KeyCode::Char('f') | KeyCode::Char('F') => app.flip_flashcard(),
                        _ => info!("Go baby go go!"),
                    }
                }
            }
        }
    }
    Ok(())
}

fn render_app(frame: &mut Frame, app: &mut App) {
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
    let msg = Paragraph::new(
        "Welcome to Rashcard, the Rust Flashcard application
         [S]how [A]dd [Q]uit",
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(msg, rows[0]);
    let main_display = cols[0];
    //now we do the main panel

    match app.state {
        app::State::Idling => draw_placeholder(frame, main_display),
        app::State::ShowFlashcard => draw_placeholder(frame, main_display),
        app::State::FlipFlashcard => draw_placeholder(frame, main_display),
        app::State::AddFlashcard => display_add_flashcard(frame, main_display, app),
        app::State::DisplaySavedPopup => {
            // info!("Saved! About to display the same");
            //TODO overlay this on the text area
            draw_saved_popup(frame).unwrap();
            app.close_popup_if_it_is_time();
        }
    }
}

///It's a placeholder
fn draw_placeholder(frame: &mut Frame, rect: Rect) {
    let msg = Paragraph::new("Placeholder-holder-holder-holder-der-r").block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(msg, rect);
}
///Currently a placeholder
fn display_in_main_window(maybe_msg: Option<&str>) -> Result<()> {
    let msg = if let Some(passed_in_msg) = maybe_msg {
        passed_in_msg
    } else {
        ""
    };

    let paragraph = Paragraph::new(msg).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
    );

    Ok(())
}

fn draw_saved_popup(f: &mut Frame) -> Result<()> {
    display_popup("Saved", f)
}

fn display_popup(msg: &str, f: &mut Frame) -> Result<()> {
    let msg = Paragraph::new(msg).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray)),
    );

    //TODO some kind of centered rect method
    //
    let rect = centered_rect(20, 20, f.size());
    f.render_widget(msg, rect);
    Ok(())
}

///Will display the text area we keep in our app for just this occasion
fn display_add_flashcard(frame: &mut Frame, rect: Rect, app: &mut App) {
    // app.init_input_area();
    frame.render_widget(app.input_area.widget(), rect)
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

#[instrument]
fn save_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    // println!("About to save the flash card");
    //get the text from app
    let lines: Vec<String> = app.input_area.clone().into_lines();
    //got nothing? do nothing
    if lines.len() < 1 {
        return Ok(());
    }
    //we have at least one line
    //top line is title
    let title = lines.get(0).unwrap();
    //
    //everything else is body
    let body = &lines[1..]
        .iter()
        .map(|line| format!("{}{}", line, "\n"))
        .collect::<String>();

    db::save_flashcard(title, body, conn)?;

    app.display_saved_popup();
    Ok(())
}

fn show_next_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    //get the next flashcard
    let offset = app.current_flashcard_number;
    let txt = if let Some(flash) = db::next_flashcard(offset, conn)? {
        //we'll append everything to the title and bring it back
        let mut title = flash.title;
        let body = flash.body;
        title.push('\n');
        title.push_str(&body);
        &title
    } else {
        app.reset_count();
        "No flashcards"
    };

    app.update_flashcard(txt);
    Ok(())
}
