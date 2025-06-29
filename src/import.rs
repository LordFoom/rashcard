use std::io::{BufRead, BufReader, Read};

use crate::db::{save_flashcard_object, FlashCard};
use anyhow::Result;
use rusqlite::Connection;

///Import a file using the Yomu export format
///Top line will be used as the title for flashcards, prefixed with a monotonically increasing
///number       
pub fn import_yomu_quotes(fp: &str, conn: &Connection) -> Result<()> {
    let file = std::fs::File::open(fp)?;
    let mut reader = BufReader::new(file);

    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let (title, author) = extract_yomu_title_author(&first_line);
    let mut file_contents = String::new();
    reader.read_to_string(&mut file_contents)?;

    if let Some(flash_cards) = extract_yomu_flashcards(&title, file_contents) {}
    Ok(())
}

pub fn extract_yomu_title_author(line: &str) -> (String, String) {
    // # An Inquiry into the Nature and Causes of the Wealth of Nations (Adam Smith)
    let mut title = String::new();
    let mut author = String::new();
    line.replace("#", "")
        .split('(')
        .enumerate()
        .for_each(|(idx, part)| {
            if idx == 0 {
                title = part.trim().to_owned();
            } else {
                //only expect two items
                author = part.replace(")", "").trim().to_owned()
            }
        });
    (title, author)
}

pub fn extract_yomu_flashcards(title: &str, file_contents: String) -> Result<Vec<FlashCard>> {
    let flash_cards = file_contents
        .split("---")
        .map(|part| {
            let body = part
                .lines()
                .filter(|line| line.starts_with(">"))
                .map(|line| line.trim_start_matches(">"))
                .collect::<Vec<_>>()
                .join("\n");
            FlashCard {
                id: 0,
                title: title.to_owned(),
                body,
            }
        })
        .collect::<Vec<FlashCard>>();
    Ok(flash_cards)
}

///Import a file into the flashcards using the ReadEra exported format
///Top line will be used as the title for flashcards, prefixed with a monotonically increasing
///number
pub fn import_read_era_quotes(fp: &str, conn: &Connection) -> Result<()> {
    let file_contents = std::fs::read_to_string(fp)?;
    //now we parse the file contents
    extract_flash_cards(file_contents)?
        .into_iter()
        .try_for_each(|flashcard| -> Result<()> { save_flashcard_object(&flashcard, conn) })?;

    Ok(())
}

///Take readera style exported notes and extract them as flashcard objects
fn extract_flash_cards(file_contents: String) -> Result<Vec<FlashCard>> {
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

        let mut flash_card_title = String::new();
        flash_card_title.push_str(&title);
        flash_card_title.push('\n');
        flash_card_title.push_str(&author);
        let fc = FlashCard {
            id: 0, //this will be ignored
            title: flash_card_title,
            body,
        };
        fcards.push(fc);
    }
    Ok(fcards)
}

mod test {
    use crate::import::extract_flash_cards;

    #[test]
    pub fn test_extract_flash_cards() {
        let text = r"Test title
            test author
this is test line one.
This is test line two.
*****
      second flascard line one.
  second Flashcard line two.
Second flashcard line 3..
*****
Third flashcard, foomy,
Foombletoning
Fumbleturning
Sevenslurring
Underscarring
--"
        .to_string();
        let flashcards = extract_flash_cards(text).unwrap();
        assert_eq!(3, flashcards.len());
        println!("flashcards! {:?}", flashcards);
    }
}
