use clap::Parser;
use std::env;
use std::fs;
use std::io;
use inquire::Select;

/// CLI tool to mark hledger transactions as cleared by matching them to a CSV file.
#[derive(Parser)]
#[command(name = "hledger_clear", version = "0.1.0", author = "Ian Wilson (uid0")]
#[command(about = "Mark hledger transactions as cleared by matching CSV files")]
struct Cli {
    /// Path to the hledger journal file. If not provided, defaults to the LEDGER_FILE environment variable.
    #[arg(short, long)]
    journal: Option<String>,

    /// Path to the CSV file
    #[arg(short, long)]
    csv: String,

    /// Output file for the updated journal
    #[arg(short, long, default_value = "updated.journal")]
    output: String,
}

fn process_files(journal_path: &str, csv_path: &str, output_path: &str) -> io::Result<()> {
    let journal_content = fs::read_to_string(journal_path)?;
    let mut journal_lines: Vec<String> = journal_content.lines().map(String::from).collect();

    let csv_content = fs::read_to_string(csv_path)?;
    let csv_records: Vec<Vec<String>> = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes())
        .deserialize()
        .filter_map(|result| result.ok())
        .collect();

    for record in csv_records {
        let date = &record[0];
        let description = &record[1];
        let amount = &record[2];

        println!(
            "CSV Transaction: Date: {}, Description: {}, Amount: {}",
            date, description, amount
        );

        let match_indices: Vec<usize> = journal_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(date) && line.contains(amount))
            .map(|(index, _)| index)
            .collect();

        if match_indices.is_empty() {
            println!("No matching transaction found in journal.");
            continue;
        }

        let options: Vec<&str> = match_indices
            .iter()
            .map(|&index| journal_lines[index].as_str())
            .collect();
        let selected = Select::new("Match a transaction:", options)
            .prompt()
            .unwrap_or_else(|_| "Skip");

        if selected == "Skip" {
            println!("Skipped transaction.");
            continue;
        }

        if let Some(&selected_index) = match_indices
            .iter()
            .find(|&&index| journal_lines[index] == selected)
        {
            journal_lines[selected_index] =
                format!("* {}", journal_lines[selected_index].trim_start());
            println!(
                "Marked transaction as cleared: {}",
                journal_lines[selected_index]
            );
        }
    }

    fs::write(output_path, journal_lines.join("\n"))?;
    println!("Updated journal written to {}", output_path);

    Ok(())
}

fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Determine journal file path
    let journal_path = cli
        .journal
        .or_else(|| env::var("LEDGER_FILE").ok())
        .unwrap_or_else(|| {
            eprintln!(
                "Error: No journal file specified and LEDGER_FILE environment variable is not set."
            );
            std::process::exit(1);
        });

    // Read the journal file
    let journal = fs::read_to_string(&journal_path).expect("Could not read journal file");

    // Process the journal (example logic)
    println!(
        "Processing journal: {}, CSV: {}, Output: {}",
        journal_path, cli.csv, cli.output
    );

    if let Err(err) = process_files(&journal_path, &cli.csv, &cli.output) {
        eprintln!("Error processing files: {}", err);
        std::process::exit(1);
    }

    // Write the updated journal back to the file
    fs::write(&journal_path, journal).expect("Could not write journal file");
}
