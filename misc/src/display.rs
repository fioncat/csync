use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::ValueEnum;
use serde::{de::DeserializeOwned, Serialize};

use crate::{api::ListResponse, table::Table};

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

pub fn pretty_json<T: Serialize>(o: T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&o)?);
    Ok(())
}

pub fn display_list<T>(
    list: ListResponse<T>,
    page: u64,
    page_size: u64,
    style: DisplayStyle,
    headless: bool,
    csv_titles: Option<String>,
) -> Result<()>
where
    T: Serialize + DeserializeOwned + TerminalDisplay,
{
    match style {
        DisplayStyle::Table => {
            if list.items.is_empty() {
                println!("<empty list>");
                return Ok(());
            }
            let mut table = Table::with_capacity(list.items.len(), headless);
            let titles = T::table_titles();
            table.add(titles.iter().map(|s| s.to_string()).collect());

            for item in list.items {
                let row = item.table_row();
                table.add(row);
            }

            let total_pages = list.total.div_ceil(page_size);

            println!("Page: {page}/{total_pages}, Total: {}", list.total);
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
            for item in list.items {
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
