use std::fs::File;

use anyhow::Result;
///Import a file into the flashcards using the ReadEra exported format
///Top line will be used as the title for flashcards, prefixed with a monotonically increasing
///number
pub fn import_read_era_quotes(fp: &str) -> Result<()> {
    let file_contents = std::fs::read_to_string(fp)?;
    //now we parse the file contents
    //first line: title
    //second line: author
    //each entry is separated by
    //*****
    for flash_card in file_contents.split("*****") {}

    Ok(())
}
