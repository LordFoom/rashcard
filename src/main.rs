use anyhow::{Context, Result};
use app::{Select, State};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use db::{default_connection, fetch_initial_flash_card_count};
use import::import_read_era_quotes;
use log::{info, LevelFilter};
use ratatui::prelude::*;
use rusqlite::Connection;
use std::io::{stdout, Stdout};
use std::time::{Duration, Instant};
// use tracing::{info, instrument, Level};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use tui_textarea::{Input, Key};

use crate::app::{App, Timer};
use crate::db::init_table;

mod app;
mod db;
mod import;
mod ui;

///Command line arguments for clap
#[derive(Parser)]
#[command(
    author = "foom",
    version = "0.1",
    about = "Flashcards in rust",
    long_about = "Flashcard to make knowledge stick like rust to metal"
)]
pub struct Args {
    ///How much to spew to the file
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbosity: u8,
    ///Is there a markdown file to read text from?
    #[arg(short, long)]
    file: Option<String>,
    ///Display random flashcard every N seconds
    #[arg(short, long)]
    timer: Option<usize>,
}

fn init_logging(level: u8) -> Result<()> {
    let lvl = match level {
        0 => LevelFilter::Error, //rock solid confidence
        1 => LevelFilter::Info,  //wibble
        2 => LevelFilter::Debug, //wobble
        _ => LevelFilter::Trace, //you are the crazy tracer man
    };
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("rashcard.log")?;
    //i wanna be a paperback I mean nonblocicking wriiiter
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(lvl))?;

    log4rs::init_config(config)?;

    info!("Initted logging");
    Ok(())
}

///TODO add ability to delete a flashcard
///TODO Timer to show cards in time
///TODO Set timer mode (forward, backward, random)
fn main() -> Result<()> {
    let args = Args::parse();
    let app = App::from_arguments(&args);
    init_logging(app.verbosity.clone())?;

    let conn = default_connection().context("failed to get sql connection")?;
    init_table(&conn)?;
    if let Some(file) = args.file {
        import_read_era_quotes(&file, &conn)?;
        println!("Imported flashcards from {}", file);
        return Ok(());
    }
    let mut maybe_timer = if let Some(t) = args.timer {
        let start = Instant::now();
        let next_card_cycle = t;
        let timer = Timer{
            start,
            next_card_cycle
        };
        Some(timer)
    }else{
        None
    };

    let mut terminal = setup_terminal().context("setup failed")?;
    run(app, &conn, &mut maybe_timer, &mut terminal).context("failed running")?;
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
    maybe_timer: &mut Option<Timer>,
    term: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<()> {
    //create the table if need be
    let flash_card_count = fetch_initial_flash_card_count(conn)?;
    app.total_cards = flash_card_count;

    loop {
        term.draw(|f| ui::render_app(f, &mut app))?;
        read_input(&mut app, conn)?;
        if !app.running {
            // info!("Going down!");
            break;
        }
        //we want to flick through if we've been passed a timer
        if let Some(t) = maybe_timer {
            if t.start.elapsed().as_secs() > t.next_card_cycle as u64 {
                show_random_flashcard(&mut app, conn)?;
                t.start = Instant::now();
            }
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
            State::ShowFlashcard | State::Idling => {
                if let Event::Key(key) = event::read().context("event read failed")? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => app.stop_running(),
                        KeyCode::Char('a') | KeyCode::Char('A') => app.show_add_flashcard(),
                        KeyCode::Char('n') | KeyCode::Char('N') => show_next_flashcard(app, conn)?,
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            show_random_flashcard(app, conn)?
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => show_prev_flashcard(app, conn)?,
                        KeyCode::Char('f') | KeyCode::Char('F') => app.flip_flashcard(),
                        KeyCode::Char('j') | KeyCode::Char('J') => app.idle(),
                        _ => info!("Go baby go go!"),
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn save_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    // println!("About to save the flash card");
    //get the text from app
    let lines: Vec<String> = app.input_area.clone().into_lines();
    //got nothing? do nothing
    if lines.is_empty() {
        return Ok(());
    }
    //we have at least one line
    //top line is title
    let title = lines.first().unwrap();
    //
    //everything else is body
    let body = &lines[1..].join("\n");

    db::save_flashcard(title, body, conn)?;

    app.display_saved_popup();
    app.total_cards += 1;
    Ok(())
}

fn show_random_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    show_flashcard(app, conn, Select::Random)
}

fn show_next_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    show_flashcard(app, conn, Select::Next)
}

fn show_prev_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    show_flashcard(app, conn, Select::Prev)
}

fn show_flashcard(app: &mut App, conn: &Connection, state: Select) -> Result<()> {
    //get the next flashcard
    if !app.first_shown {
        app.first_shown = true;
    } else {
        match state {
            Select::Next => app.increment_flash_count(),
            Select::Prev => app.decrement_flash_count(),
            Select::Random => app.randomize_flash_count(),
        }
    };

    let offset = app.current_flashcard_number;
    info!("Current flash number: {}", offset);
    info!("State: {:?}", state);
    let txt = if let Some(flash) = db::next_flashcard(offset, conn)? {
        //we'll append everything to the title and bring it back
        let mut text = flash.title;
        let body = flash.body;
        text.push('\n');
        text.push_str(&body);
        //move onto the next flashcard
        text
    } else {
        app.reset_count();
        "No flashcards".to_owned()
    };

    app.show_flash_card();
    info!(
        "Our offset after changing the card: {}",
        app.current_flashcard_number
    );
    app.update_flash_text(&txt);
    Ok(())
}
