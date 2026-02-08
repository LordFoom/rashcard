use std::collections::HashMap;

use anyhow::{bail, Result};
use log::info;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct FlashCard {
    pub title: String,
    pub body: String,
    //db id
    pub id: usize,
}

///Titles are book titles
///Number of quotes from each title
pub struct CardTitleReport {
    pub report_lines: Vec<ReportLine>,
}

pub struct ReportLine {
    title: String,
    title_count: usize,
}

impl CardTitleReport {
    pub fn new() -> Self {
        Self {
            report_lines: Vec::new(),
        }
    }

    pub fn add_line(&mut self, report_line: ReportLine) {
        self.report_lines.push(report_line);
    }
}

pub fn default_connection() -> Result<Connection> {
    let conn = Connection::open("./.rashcard.db")?;
    Ok(conn)
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
        [title, body],
    )?;

    Ok(())
}

pub fn next_flashcard(offset: usize, conn: &Connection) -> Result<Option<FlashCard>> {
    info!("This is the offset for next flashcard: {}", offset);
    let mut qry =
        conn.prepare("SELECT id, title, body FROM flashcard ORDER BY id LIMIT 1 OFFSET ?")?;
    let flashcards = qry.query_map(params![offset], |row| {
        Ok(FlashCard {
            id: row.get(0)?,
            title: row.get(1)?,
            body: row.get(2)?,
        })
    })?;

    let mut flashcard = None;
    //should only be one in here
    for maybe_fc in flashcards {
        if flashcard.is_none() {
            flashcard = Some(maybe_fc.unwrap());
        } else {
            bail!("Expected only 1 flashcard, found at least 2");
        }
    }
    //
    Ok(flashcard)
}

pub fn delete_flashcard(fc_id: usize, conn: &Connection) -> Result<()> {
    conn.execute("DELETE from flashcard where id = ?1", [&fc_id.to_string()])?;
    Ok(())
}

pub fn construct_title_report(conn: &Connection) -> Result<CardTitleReport> {
    let mut qry = conn.prepare("SELECT title, COUNT(*) FROM flashcard GROUP BY title")?;
    let mut report = CardTitleReport::new();
    qry.query_map([], |row| {
        report.add_line(ReportLine {
            title: row.get(0)?,
            title_count: row.get(1)?,
        });
        Ok(())
    })?;
    Ok(report)
}
