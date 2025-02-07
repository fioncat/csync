use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::ValueEnum;
use serde::Serialize;

use crate::table::Table;

/// Display style options for output formatting
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DisplayStyle {
    /// Display data in a formatted table
    Table,
    /// Display data in JSON format
    Json,
    /// Display data in CSV format
    Csv,
}

/// Trait for types that can be displayed in terminal with different formats
pub trait TerminalDisplay {
    /// Returns the column titles for table display
    fn table_titles() -> Vec<&'static str>;
    /// Converts the instance into a row of strings for table display
    fn table_row(self) -> Vec<String>;

    /// Returns the column titles for CSV display
    fn csv_titles() -> Vec<&'static str>;
    /// Converts the instance into a map of field name to value for CSV display
    fn csv_row(self) -> HashMap<&'static str, String>;
}

/// Displays an object as formatted JSON
///
/// # Arguments
/// * `o` - Any object that implements Serialize
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err` if JSON serialization fails
pub fn display_json<T: Serialize>(o: T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&o)?);
    Ok(())
}

/// Displays a list of items in the specified format
///
/// # Arguments
/// * `list` - Vector of items to display
/// * `style` - Output format (Table, JSON, or CSV)
/// * `headless` - If true, omits headers in the output
/// * `csv_titles` - Optional comma-separated list of columns to include in CSV output
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err` if formatting or display fails
pub fn display_list<T: Serialize + TerminalDisplay>(
    list: Vec<T>,
    style: DisplayStyle,
    headless: bool,
    csv_titles: Option<String>,
) -> Result<()> {
    match style {
        DisplayStyle::Table => {
            if list.is_empty() {
                println!("<empty list>");
                return Ok(());
            }
            let mut table = Table::with_capacity(list.len(), headless);
            let titles = T::table_titles();
            table.add(titles.iter().map(|s| s.to_string()).collect());

            for item in list {
                let row = item.table_row();
                table.add(row);
            }

            table.show();
        }
        DisplayStyle::Csv => {
            let mut titles = T::csv_titles();
            if let Some(filter) = csv_titles {
                let filter = filter.split(',').collect::<Vec<_>>();
                titles = titles
                    .iter()
                    .filter(|t| filter.contains(t))
                    .copied()
                    .collect();
            }
            if titles.is_empty() {
                bail!("No csv column to display, available: {:?}", T::csv_titles());
            }

            if !headless {
                println!("{}", titles.join(","));
            }
            for item in list {
                let mut row = item.csv_row();
                let mut values = Vec::with_capacity(titles.len());
                for title in titles.iter() {
                    let value = row.remove(*title).unwrap();
                    values.push(value);
                }
                println!("{}", values.join(","));
            }
        }
        DisplayStyle::Json => {
            let json = serde_json::to_string_pretty(&list)?;
            println!("{}", json);
        }
    }
    Ok(())
}
