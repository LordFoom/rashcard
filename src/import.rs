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

    let flash_cards = extract_flash_cards(file_contents)?;
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

        let mut flash_card_title = String::new();
        flash_card_title.push_str(&title);
        flash_card_title.push('\n');
        flash_card_title.push_str(&author);
        let fc = FlashCard{
            title: flash_card_title,
            body,
        };
        fcards.push(fc);
    }
    Ok(fcards)
}

mod test{
    use crate::import::extract_flash_cards;

    #[test]
    pub fn test_extract_flash_cards(){
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
--".to_string();
        let flashcards = extract_flash_cards(text).unwrap();
        assert_eq!(3, flashcards.len());
        println!("flashcards! {:?}", flashcards);
    }
}
