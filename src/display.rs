use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::ValueEnum;
use serde::Serialize;

use crate::table::Table;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DisplayStyle {
    Table,
    Json,
    Csv,
}

pub trait TerminalDisplay {
    fn table_titles() -> Vec<&'static str>;
    fn table_row(self) -> Vec<String>;

    fn csv_titles() -> Vec<&'static str>;
    fn csv_row(self) -> HashMap<&'static str, String>;
}

pub fn display_json<T: Serialize>(o: T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&o)?);
    Ok(())
}

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
