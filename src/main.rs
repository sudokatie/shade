//! Shade CLI - Privacy-first personal analytics

use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use shade::analytics::{compute_daily_summary, default_categories};
use shade::config::ShadeConfig;
use shade::db::Database;

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

    /// List all tracked applications
    List,

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
            let config = ShadeConfig::load()?;
            println!("Shade Status");
            println!("  Database: {:?}", config.db_path);

            if config.db_path.exists() {
                let db = Database::open(&config.db_path)?;
                let today = Utc::now().date_naive();
                let total = db.get_daily_screen_time(today)?;
                let hours = total / 3600;
                let minutes = (total % 3600) / 60;
                println!("  Today's screen time: {}h {}m", hours, minutes);
            } else {
                println!("  Database not found (run 'shade init' first)");
            }
        }

        Commands::Today => {
            let config = ShadeConfig::load()?;

            if !config.db_path.exists() {
                println!("No data yet. Run 'shade start' to begin tracking.");
                return Ok(());
            }

            let db = Database::open(&config.db_path)?;
            let today = Utc::now().date_naive();
            let categories = default_categories();
            let summary = compute_daily_summary(&db, today, Some(&categories))?;

            println!("Today's Screen Time: {}", summary.format_total_time());
            println!();

            if !summary.category_breakdown.is_empty() {
                println!("By Category:");
                for cat in &summary.category_breakdown {
                    let hours = cat.seconds / 3600;
                    let minutes = (cat.seconds % 3600) / 60;
                    println!("  {:20} {:>2}h {:>2}m", cat.category, hours, minutes);
                }
                println!();
            }

            if !summary.top_apps.is_empty() {
                println!("Top Apps:");
                for (i, app) in summary.top_apps.iter().take(5).enumerate() {
                    let hours = app.seconds / 3600;
                    let minutes = (app.seconds % 3600) / 60;
                    println!("  {}. {:20} {:>2}h {:>2}m", i + 1, app.name, hours, minutes);
                }
            }
        }

        Commands::Apps { limit, period } => {
            let config = ShadeConfig::load()?;

            if !config.db_path.exists() {
                println!("No data yet. Run 'shade start' to begin tracking.");
                return Ok(());
            }

            let db = Database::open(&config.db_path)?;
            let today = Utc::now().date_naive();

            let (start, end) = match period.as_str() {
                "week" => (today - Duration::days(7), today),
                "month" => (today - Duration::days(30), today),
                _ => (today, today), // "today" or default
            };

            let apps = db.get_top_apps(start, end, limit)?;

            println!("Top {} Apps ({})", limit, period);
            println!();

            if apps.is_empty() {
                println!("  No data for this period.");
            } else {
                for (i, (app, secs)) in apps.iter().enumerate() {
                    let hours = secs / 3600;
                    let minutes = (secs % 3600) / 60;
                    println!(
                        "  {:>2}. {:30} {:>2}h {:>2}m",
                        i + 1,
                        app.name,
                        hours,
                        minutes
                    );
                }
            }
        }

        Commands::List => {
            let config = ShadeConfig::load()?;

            if !config.db_path.exists() {
                println!("No data yet. Run 'shade start' to begin tracking.");
                return Ok(());
            }

            let db = Database::open(&config.db_path)?;
            let apps = db.get_all_applications()?;

            println!("Tracked Applications ({} total)", apps.len());
            println!();

            for app in &apps {
                let category = app.category.as_deref().unwrap_or("Uncategorized");
                println!("  {:30} [{}]", app.name, category);
                println!("    {}", app.bundle_id);
            }
        }

        Commands::Dashboard => {
            let config = ShadeConfig::load()?;

            if !config.db_path.exists() {
                println!(
                    "No data yet. Run 'shade init' first, then 'shade start' to begin tracking."
                );
                return Ok(());
            }

            shade::tui::run(config.db_path.to_str().unwrap_or(":memory:"))?;
        }

        Commands::Export { output, from, to } => {
            let config = ShadeConfig::load()?;

            if !config.db_path.exists() {
                println!("No data yet. Run 'shade init' first.");
                return Ok(());
            }

            let db = Database::open(&config.db_path)?;
            let today = Utc::now().date_naive();

            // Parse dates or use defaults
            let start = match from {
                Some(s) => s
                    .parse::<chrono::NaiveDate>()
                    .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?,
                None => today - Duration::days(30), // Default to last 30 days
            };

            let end = match to {
                Some(s) => s
                    .parse::<chrono::NaiveDate>()
                    .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?,
                None => today,
            };

            println!("Exporting data from {} to {}...", start, end);

            shade::export::export_to_file(&db, start, end, &output)?;

            println!("Exported to {:?}", output);
        }

        Commands::Init => {
            let config = ShadeConfig::default();

            // Create config directory and database directory
            if let Some(parent) = config.db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Initialize database
            let db = Database::open(&config.db_path)?;
            drop(db); // Close connection

            config.save()?;

            println!("Shade initialized!");
            println!("  Config: ~/.shade/config.yaml");
            println!("  Database: {:?}", config.db_path);
            println!();
            println!("Run 'shade start' to begin tracking.");
        }
    }

    Ok(())
}
