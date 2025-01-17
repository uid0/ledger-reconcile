use clap::Parser;
use csv;
use inquire::Select;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

/// CLI tool to mark hledger transactions as cleared by matching them to a CSV file.
#[derive(Parser)]
#[command(name = "hledger_clear", version = "0.1.0", author = "Ian Wilson (uid0")]
#[command(about = "Mark hledger transactions as cleared by matching CSV files")]
struct Cli {
    /// Path to the ledger file
    #[arg(short, long)]
    ledger: Option<String>,

    /// Path to the CSV file
    #[arg(short, long)]
    csv: String,

    /// Output file for the updated ledger
    #[arg(short, long, default_value = "updated.ledger")]
    output: String,
}

/// Read and process the ledger and CSV files, then match transactions interactively.
fn process_files(ledger_path: &str, csv_path: &str, output_path: &str) -> io::Result<()> {
    let ledger_content = fs::read_to_string(ledger_path)?;
    let mut ledger_lines: Vec<String> = ledger_content.lines().map(String::from).collect();

    // Filter out cleared transactions (marked with "*")
    let uncleared_lines: Vec<_> = ledger_lines
        .windows(3)
        .filter(|block| !block[0].starts_with("*"))
        .map(|block| block.join("\n"))
        .collect();

    let csv_content = fs::read_to_string(csv_path)?;
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());
    let csv_records: Vec<Vec<String>> = csv_reader
        .records()
        .filter_map(|result| result.ok())
        .map(|record| record.iter().map(String::from).collect())
        .collect();

    for record in csv_records {
        let date = record[0].trim();
        let description = record[1].trim().to_lowercase();
        let amount = record[2].trim().replace('$', "");

        println!("\n--- CSV Transaction ---");
        println!(
            "Date: {}, Description: {}, Amount: {}",
            date, description, amount
        );
        println!("-----------------------");

        let matches: Vec<(usize, String)> = uncleared_lines
            .iter()
            .enumerate()
            .filter_map(|(index, combined)| {
                let normalized = combined.to_lowercase().replace('$', "");

                if normalized.contains(date)
                    && normalized.contains(&description)
                    && normalized.contains(&amount)
                {
                    Some((index, combined.clone()))
                } else {
                    None
                }
            })
            .collect();

        if matches.is_empty() {
            println!("No matching transaction found in ledger.");

            let action = Select::new(
                "What would you like to do?",
                vec!["Ignore", "Add Stock Expense Item", "Exit"],
            )
            .prompt()
            .unwrap_or_else(|_| "Ignore");

            if action == "Add Stock Expense Item" {
                let new_entry = format!(
                    "{} {}
    Expenses:Miscellaneous          ${}
    Assets:Bank                    -${}",
                    date, description, amount, amount
                );
                ledger_lines.push(new_entry.clone());
                println!("Added new transaction to ledger:");
                println!("{}", new_entry);
            } else if action == "Exit" {
                println!("Exiting program.");
                fs::write(output_path, ledger_lines.join("\n"))?;
                println!("Updated ledger written to {}", output_path);
                return Ok(());
            } else {
                println!("Ignored this transaction.");
            }

            continue;
        }

        println!("\n--- Matching Ledger Entries ---");
        for (i, (_, block)) in matches.iter().enumerate() {
            println!("{}. {}", i + 1, block);
        }
        println!("-------------------------------");

        let mut options: Vec<&str> = matches.iter().map(|(_, block)| block.as_str()).collect();
        options.push("Ignore this line");

        let selected = Select::new("Match a transaction:", options)
            .prompt()
            .unwrap_or_else(|_| "Ignore this line");

        if selected == "Ignore this line" {
            println!("Skipped transaction.");
            continue;
        }

        if let Some(&(selected_index, _)) =
            matches.iter().find(|(_, block)| block.as_str() == selected)
        {
            let start_line = selected_index;
            ledger_lines[start_line] = format!("* {}", ledger_lines[start_line].trim_start());
            println!(
                "Marked transaction as cleared: {}",
                ledger_lines[start_line]
            );
        }
    }

    fs::write(output_path, ledger_lines.join("\n"))?;
    println!("Updated ledger written to {}", output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_process_files() {
        let ledger_path = "test_ledger.ledger";
        let csv_path = "test_transactions.csv";
        let output_path = "test_updated.ledger";

        let mut ledger_file = File::create(ledger_path).unwrap();
        writeln!(
            ledger_file,
            r#"
2025-01-01 Groceries
    Expenses:Food          $50.00
    Assets:Bank           -$50.00

2025-01-02 Rent
    Expenses:Rent         $1000.00
    Assets:Bank          -$1000.00
"#
        )
        .unwrap();

        let mut csv_file = File::create(csv_path).unwrap();
        writeln!(
            csv_file,
            r#"
Date,Description,Amount
2025-01-01,Groceries,$50.00
2025-01-02,Rent,$1000.00
"#
        )
        .unwrap();

        process_files(ledger_path, csv_path, output_path).unwrap();

        let updated_ledger = fs::read_to_string(output_path).unwrap();
        assert!(updated_ledger.contains("* 2025-01-01 Groceries"));
        assert!(updated_ledger.contains("* 2025-01-02 Rent"));

        fs::remove_file(ledger_path).unwrap();
        fs::remove_file(csv_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }
}

fn main() {
    let cli = Cli::parse();

    let ledger_path = cli
        .ledger
        .or_else(|| env::var("LEDGER_FILE").ok())
        .unwrap_or_else(|| {
            eprintln!(
                "Error: No ledger file specified and LEDGER_FILE environment variable is not set."
            );
            std::process::exit(1);
        });

    if let Err(err) = process_files(&ledger_path, &cli.csv, &cli.output) {
        eprintln!("Error processing files: {}", err);
        std::process::exit(1);
    }
}
