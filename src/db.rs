use anyhow::{bail, Result};
use rusqlite::{params, Connection};

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

pub fn save_flashcard(title: &str, body: &str, conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO flashcard(title, body) values (?1, ?2)",
        &[title, body],
    )?;

    Ok(())
}

pub fn next_flashcard(offset: usize, conn: &Connection) -> Result<Option<FlashCard>> {
    let mut qry =
        conn.prepare("SELECT title, body FROM flashcard ORDER BY id LIMIT 1 OFFSET ?")?;
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
