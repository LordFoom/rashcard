use anyhow::{Context, Result};
use app::{FlashCardMode, Select, State};
use arboard::Clipboard;
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
    version = "1.1",
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
    ///Display default random flashcard every N seconds
    #[arg(short, long)]
    timer: Option<usize>,
    ///Set display mode for timer: Forward, Backward, Random
    #[arg(short, long, requires("timer"), value_enum)]
    mode: Option<FlashCardMode>,
    //A sound file to play when a timer reaches the end
    sound: Option<String>,
}

fn init_logging(level: u8) -> Result<()> {
    let lvl = match level {
        0 => LevelFilter::Error, //rock solid confidence
        1 => LevelFilter::Info,  //wibble
        2 => LevelFilter::Debug, //wobble
        _ => LevelFilter::Trace, //you are the crazy tracer man
    };
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} - {m}{n}")))
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
///TODO Add open file dialog
///TODO add plugin framework for formats
///TODO convert readme reading into plugin
/// Rash: obsolete definition : quickly effective
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
    let mut maybe_timer = maybe_construct_timer(&args);
    let mut terminal = setup_terminal().context("setup failed")?;
    run(app, &conn, &mut maybe_timer, &mut terminal).context("failed running")?;
    // tracing::debug!()
    // let mut terminal = Terminal::new(CrosstermBackend::new(stdout()));
    unsetup_terminal(&mut terminal).context("unsetup failed")
}

fn maybe_construct_timer(args: &Args) -> Option<Timer> {
    if let Some(t) = args.timer {
        let start = Instant::now();
        let next_card_cycle = t;
        let draw_mode = if let Some(mode) = args.mode.clone() {
            mode
        } else {
            FlashCardMode::Random
        };

        let timer = Timer {
            start,
            next_card_cycle,
            draw_mode,
        };
        info!("We have a timer! {}s", t);
        Some(timer)
    } else {
        None
    }
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
            break;
        }
        //we want to flick through if we've been passed a timer
        if let Some(t) = maybe_timer {
            if t.start.elapsed().as_secs() > t.next_card_cycle as u64 {
                match t.draw_mode {
                    FlashCardMode::Forward => show_next_flashcard(&mut app, conn)?,
                    FlashCardMode::Backward => show_prev_flashcard(&mut app, conn)?,
                    FlashCardMode::Random => show_random_flashcard(&mut app, conn)?,
                }
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
                    key: Key::Char('b'),
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
                        KeyCode::Char('b') | KeyCode::Char('B') => app.idle(),
                        KeyCode::Char('j')
                        | KeyCode::Char('J')
                        | KeyCode::Down
                        | KeyCode::PageDown => app.scroll_down(),
                        KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up | KeyCode::PageUp => {
                            app.scroll_up()
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            maybe_delete_flashcard(app, conn)?
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            copy_flashcard_to_clipboard(app)?
                        }
                        // KeyCode
                        _ => info!("Go baby go go!"),
                    }
                }
            }
            State::DisplayDeletePopup => {
                if let Event::Key(key) = event::read().context("event read failed")?.into() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            actually_delete_flashcard(app, conn)?
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            info!("Decided not to delete flashcard");
                            app.restore_prior_state();
                        }
                        //do nothing in all other cases
                        _ => {}
                    }
                }
            }
            //all the other states don't need no stinking input
            _ => {}
        }
    }
    Ok(())
}

///Copy the currently displayed flashcard to the clipboard
///VI till I die
fn copy_flashcard_to_clipboard(app: &mut App) -> Result<()> {
    if app.has_flashcards() {
        let mut clip = Clipboard::new()?;
        let txt = app.current_flash_text.clone();
        clip.set_text(txt)?;
        app.visual_flicker = true;
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
        let mut text = flash.title;
        let body = flash.body;
        text.push('\n');
        text.push_str(&body);
        app.current_flashcard_id = flash.id;
        text
    } else {
        app.reset_count();
        "No flashcards".to_owned()
    };

    info!(
        "Our offset after changing the card: {}",
        app.current_flashcard_number
    );
    app.update_flash_text(&txt);
    app.reset_scrollbar_state();
    app.increment_display_count();
    app.show_flash_card();
    Ok(())
}

///This will cause a "Really delete" modal to display
fn maybe_delete_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    info!("Maybe deleting a flashcard!");
    if app.has_flashcards() {
        //show the confirm delete dialog
        app.start_delete();
    }

    Ok(())
}

fn actually_delete_flashcard(app: &mut App, conn: &Connection) -> Result<()> {
    info!("Actually going to delete a flashcard!");
    let curr_id = app.current_flashcard_id;
    if app.has_flashcards() {
        //show the confirm delete dialog
        db::delete_flashcard(app.current_flashcard_id, conn)?;
        app.total_cards -= 1;
    }
    info!("Deleted flashcard with id {}", curr_id);
    show_prev_flashcard(app, conn)?;

    Ok(())
}
