#![cfg(not(windows))]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use crate::state::AppState;

pub(crate) fn run_terminal_demo(
    state: Arc<Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Cross-platform interactive demo mode.");
    println!("Type a query to search, '=expr' for math, or ':q' to quit.");

    loop {
        print!("Spotlight > ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }

        let query = input.trim();
        if query == ":q" || query == "exit" {
            break;
        }

        let start_search = std::time::Instant::now();
        let mut app_state = state.lock().unwrap();
        app_state.set_query(query.to_string());
        let search_duration = start_search.elapsed();

        println!(
            "Found {} in {:.3?}",
            app_state.results.len(),
            search_duration
        );

        if app_state.results.is_empty() {
            println!("  No matching applications or calculations found.");
        } else {
            for (i, res) in app_state.results.iter().enumerate() {
                let sub = res.subtitle.as_deref().unwrap_or("");
                println!("  [{}] {:<25} | {}", i + 1, res.title, sub);
            }
        }
        println!();
    }

    Ok(())
}
