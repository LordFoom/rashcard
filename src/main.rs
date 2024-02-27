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
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use rusqlite::Connection;
use std::time::Duration;
use std::{
    io::{stdout, Stdout},
    process::exit,
};
// use tracing::{info, instrument, Level};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use tui_textarea::{Input, Key};

use crate::app::App;
use crate::db::init_table;

mod app;
mod db;
mod import;

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
}

fn init_logging(level: u8) -> Result<()> {
    //TODO set this via config
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

///TODO Read in flashcards from cli
///TODO read in flashcards from markdown files
///TODO add ability to delete a flashcard
fn main() -> Result<()> {
    let args = Args::parse();
    let app = App::from_arguments(&args);
    init_logging(app.verbosity.clone())?;

    //TODO finish importing
    let conn = default_connection().context("failed to get sql connection")?;
    init_table(&conn)?;
    if let Some(file) = args.file {
        import_read_era_quotes(&file, &conn)?;
        println!("Imported flashcards from {}", file);
        return Ok(());
    }
    let mut terminal = setup_terminal().context("setup failed")?;
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
    let flash_card_count = fetch_initial_flash_card_count(conn)?;
    app.total_cards = flash_card_count;

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
            State::ShowFlashcard | State::Idling => {
                if let Event::Key(key) = event::read().context("event read failed")? {
                    //TODO we need to make the first show go false again when  not showing
                    //flashcards
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => app.stop_running(),
                        KeyCode::Char('a') | KeyCode::Char('A') => app.show_add_flashcard(),
                        KeyCode::Char('n') | KeyCode::Char('N') => show_next_flashcard(app, conn)?,
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
         [N]ext | [P]revious | [A]dd | [Q]uit",
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
        app::State::ShowFlashcard => display_current_flashcard(frame, main_display, app),
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

fn display_current_flashcard(frame: &mut Frame, rect: Rect, app: &mut App) {
    let text = &app.current_flash_text;
    let msg = Paragraph::new(&text[..])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(msg, rect);
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
            Select::Random => panic!("How did we get here? Random not yet supported"),
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
