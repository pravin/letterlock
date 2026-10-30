use std::process;

use letterlock::{config, gameui, stats};

fn main() {
    // Parse command-line arguments
    let config = config::Config::parse_args();

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration error: {}", e);
        process::exit(2);
    }

    let mut stats_manager = stats::StatsManager::new();

    // Handle special commands
    if config.stats {
        for line in gameui::format_stats(stats_manager.get_stats(), config.max_guesses) {
            println!("{}", line);
        }
        if let Some(path) = stats_manager.file_path() {
            println!("\nStats file: {}", path.display());
        }
        return;
    }

    if config.reset_stats {
        if let Err(e) = stats_manager.reset_stats() {
            eprintln!("Error resetting statistics: {}", e);
            process::exit(1);
        }
        println!("Statistics reset successfully!");
        return;
    }

    if let Err(e) = gameui::run(&config, &mut stats_manager) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
