use anyhow::Result;
use colored::Colorize;

use crate::db;

pub fn print_out_report(title_report: &db::CardTitleReport) -> Result<()> {
    println!("{}", "Report on titles".red());
    println!("{}", "=================".magenta());
    title_report.report_lines.iter().for_each(|line| {
        println!("{} -> {}", line.title, line.title_count);
    });
    Ok(())
}
