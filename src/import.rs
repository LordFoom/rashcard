use std::fs::File;

use anyhow::Result;
use rusqlite::Connection;
use crate::db::FlashCard;

///Import a file into the flashcards using the ReadEra exported format
///Top line will be used as the title for flashcards, prefixed with a monotonically increasing
///number
pub fn import_read_era_quotes(fp: &str, conn: &Connection) -> Result<()> {
    let file_contents = std::fs::read_to_string(fp)?;
    //now we parse the file contents
    //first line: title
    //second line: author
    //rest is entries

    extract_flash_cards(file_contents);
    //TODO here wi stick it into the db, will need to pass in conn
    //OOOOR do we return the collection?

    Ok(())
}

fn extract_flash_cards(file_contents: String) -> Result<Vec<FlashCard>>{
    let mut title = String::new();
    let mut author = String::new();

    //each entry is separated by
    //*****
    let mut fcards = Vec::new();
    for (i, flash_card) in file_contents.split("*****").enumerate() {
        let mut body = String::new();
        if i == 0 {
            for (j, line) in flash_card.split('\n').enumerate() {
                //book title
                if j == 0 {
                    title = line.to_string();
                } else if j == 1 {
                    //author
                    author = line.to_string(); //
                } else {
                    body.push_str(line);
                    body.push('\n');
                }
            }
        } else {
            //not the title card
            body = flash_card.to_string();
        }
        let fc = FlashCard{
            title: title.clone(),
            body,
        };
        fcards.push(fc);
    }
    Ok(fcards)
}

mod test{

    #[test]
    pub fn test_
}
