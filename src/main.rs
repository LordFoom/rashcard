use anyhow::{Context, Result};
use app::State;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::io::{stdout, Stdout};
use std::time::Duration;
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
    run(app, &mut terminal).context("failed running")?;
    // tracing::debug!()
    // let mut terminal = Terminal::new(CrosstermBackend::new(stdout()));
    unsetup_terminal(&mut terminal).context("unsetup failed")
}

#[instrument]
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    info!("Setting up terminal...");
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("unable to enter alternate screen")?;
    let term = Terminal::new(CrosstermBackend::new(stdout)).context("unable to setup terminal");
    info!("Terminal setup");
    term
}

#[instrument]
fn unsetup_terminal(term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    info!("Unsetting up terminal...");
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(term.backend_mut(), LeaveAlternateScreen)
        .context("unable to return to main screen")?;
    term.show_cursor().context("unable to show cursor")
}

#[instrument]
fn run(mut app: App, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    loop {
        term.draw(|f| render_app(f, &mut app))?;
        read_input(&mut app)?;
        if !app.running {
            info!("Going down!");
            break;
        }
    }
    Ok(())
}

fn read_input(app: &mut App) -> Result<()> {
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
                } => app.show_next_flashcard(),

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
                        KeyCode::Char('s') | KeyCode::Char('S') => app.show_next_flashcard(),
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
    }
    //
    //
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

///Will display the text area we keep in our app for just this occasion
fn display_add_flashcard(frame: &mut Frame, rect: Rect, app: &mut App) {
    // app.init_input_area();
    frame.render_widget(app.input_area.widget(), rect)
}
