use anyhow::{Context, Result};
use rusqlite::Connection;

pub fn default_connection() -> Result<Connection> {
    let conn = Connection::open("./.rashcard.db")?;
    return Ok(conn);
}

pub fn init_table(conn: &Connection) -> Result<()> {
    conn.execute(
        r"CREATE TABLE IF NOT EXISTS flashcard 
                 (id INT PRIMARY KEY ASC,
                  title TEXT,
                  body TEXT,
                  create_date TEXT DEFAULT CURRENT_TIMESTAMP,
                  last_update TEXT DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    Ok(())
}
