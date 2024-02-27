use anyhow::{bail, Result};
use log::info;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct FlashCard {
    pub title: String,
    pub body: String,
}

pub fn default_connection() -> Result<Connection> {
    let conn = Connection::open("./.rashcard.db")?;
    return Ok(conn);
}

pub fn init_table(conn: &Connection) -> Result<()> {
    conn.execute(
        r"CREATE TABLE IF NOT EXISTS flashcard 
                 (id INTEGER PRIMARY KEY,
                  title TEXT,
                  body TEXT,
                  create_date TEXT DEFAULT CURRENT_TIMESTAMP,
                  last_update TEXT DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    Ok(())
}

pub fn fetch_initial_flash_card_count(conn: &Connection) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM flashcard")?;
    let mut rows = stmt.query([])?;
    let mut count = 0;
    if let Some(row) = rows.next()? {
        count = row.get(0)?;
    }
    Ok(count)
}

pub fn save_flashcard_object(fc: &FlashCard, conn: &Connection) -> Result<()> {
    save_flashcard(&fc.title, &fc.body, conn)
}

pub fn save_flashcard(title: &str, body: &str, conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO flashcard(title, body) values (?1, ?2)",
        &[title, body],
    )?;

    Ok(())
}

pub fn next_flashcard(offset: usize, conn: &Connection) -> Result<Option<FlashCard>> {
    info!("This is the offset for next flashcard: {}", offset);
    let mut qry = conn.prepare("SELECT title, body FROM flashcard ORDER BY id LIMIT 1 OFFSET ?")?;
    let flashcards = qry.query_map(params![offset], |row| {
        Ok(FlashCard {
            title: row.get(0)?,
            body: row.get(1)?,
        })
    })?;

    let mut flashcard = None;
    //should only be one in here
    for maybe_fc in flashcards {
        if let None = flashcard {
            flashcard = Some(maybe_fc.unwrap());
        } else {
            bail!("Expected only 1 flashcard, found at least 2");
        }
    }
    //
    Ok(flashcard)
}
