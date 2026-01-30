//! Shade CLI - Privacy-first personal analytics

use clap::{Parser, Subcommand};
use shade::config::ShadeConfig;

#[derive(Parser)]
#[command(name = "shade")]
#[command(about = "Privacy-first personal analytics. All data local.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the collection daemon
    Start,
    
    /// Stop the collection daemon
    Stop,
    
    /// Show daemon status
    Status,
    
    /// Show today's screen time summary
    Today,
    
    /// Show top apps by usage
    Apps {
        /// Number of apps to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
        
        /// Time period (today, week, month)
        #[arg(short, long, default_value = "today")]
        period: String,
    },
    
    /// Open the TUI dashboard
    Dashboard,
    
    /// Export data to JSON
    Export {
        /// Output file path
        #[arg(short, long)]
        output: std::path::PathBuf,
        
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,
        
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,
    },
    
    /// Initialize with example config
    Init,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start => {
            println!("Starting Shade daemon...");
            let config = ShadeConfig::load()?;
            println!("  Database: {:?}", config.db_path);
            println!("  Idle timeout: {}s", config.idle_timeout_secs);
            println!("(Daemon not yet implemented)");
        }
        
        Commands::Stop => {
            println!("Stopping Shade daemon...");
            println!("(Daemon not yet implemented)");
        }
        
        Commands::Status => {
            println!("Shade daemon status:");
            println!("  Running: false");
            println!("  Database: {:?}", ShadeConfig::default().db_path);
            println!("(Status check not yet implemented)");
        }
        
        Commands::Today => {
            println!("Today's Screen Time:");
            println!("  Total: 0h 0m");
            println!("  (Data collection not yet implemented)");
        }
        
        Commands::Apps { limit, period } => {
            println!("Top {} apps ({})", limit, period);
            println!("  (No data yet)");
        }
        
        Commands::Dashboard => {
            println!("Opening dashboard...");
            println!("(TUI not yet implemented)");
        }
        
        Commands::Export { output, from, to } => {
            println!("Exporting data to {:?}", output);
            if let Some(f) = from {
                println!("  From: {}", f);
            }
            if let Some(t) = to {
                println!("  To: {}", t);
            }
            println!("(Export not yet implemented)");
        }
        
        Commands::Init => {
            let config = ShadeConfig::default();
            config.save()?;
            println!("Created config at ~/.shade/config.yaml");
            println!("Database will be at: {:?}", config.db_path);
        }
    }
    
    Ok(())
}
