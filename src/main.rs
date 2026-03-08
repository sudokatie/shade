//! Shade CLI - Privacy-first personal analytics

use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use shade::analytics::{compute_daily_summary, default_categories, merge_categories, check_goals, WarningLevel};
use shade::config::{ShadeConfig, TimeGoal};
use shade::db::Database;
use std::collections::HashMap;

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

    /// Export data to JSON or CSV
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

        /// Export format (json or csv)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// CSV export type: daily, apps, or categories (only for csv format)
        #[arg(long, default_value = "apps")]
        csv_type: String,
    },

    /// Initialize with example config
    Init,

    /// Manage app categories
    Category {
        #[command(subcommand)]
        action: CategoryCommands,
    },

    /// Manage time goals
    Goals {
        #[command(subcommand)]
        action: GoalCommands,
    },
}

#[derive(Subcommand)]
enum CategoryCommands {
    /// List all categories
    List,

    /// Add an app to a category
    Add {
        /// Bundle ID of the app
        bundle_id: String,
        /// Category name
        category: String,
    },

    /// Remove an app from a category
    Remove {
        /// Bundle ID of the app
        bundle_id: String,
        /// Category name
        category: String,
    },

    /// Show all apps in a category
    Show {
        /// Category name
        category: String,
    },
}

#[derive(Subcommand)]
enum GoalCommands {
    /// List all time goals
    List,

    /// Add a time goal for an app or category
    Add {
        /// Target (bundle ID or category name)
        target: String,
        /// Daily limit in minutes
        limit: u32,
        /// Set goal for a category instead of app
        #[arg(short, long)]
        category: bool,
    },

    /// Remove a time goal
    Remove {
        /// Target (bundle ID or category name)
        target: String,
        /// Remove a category goal instead of app goal
        #[arg(short, long)]
        category: bool,
    },

    /// Show progress toward all goals
    Status,
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
            let user_categories = config.category_map();
            let categories = merge_categories(&user_categories, true);
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

        Commands::Export { output, from, to, format, csv_type } => {
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

            match format.to_lowercase().as_str() {
                "json" => {
                    shade::export::export_to_file(&db, start, end, &output)?;
                    println!("Exported JSON to {:?}", output);
                }
                "csv" => {
                    let export_type: shade::export::CsvExportType = csv_type
                        .parse()
                        .map_err(|e: String| anyhow::anyhow!(e))?;
                    shade::export::export_to_csv_file(&db, start, end, &output, export_type)?;
                    println!("Exported CSV ({}) to {:?}", csv_type, output);
                }
                _ => {
                    anyhow::bail!("Unknown format '{}'. Use 'json' or 'csv'.", format);
                }
            }
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

        Commands::Category { action } => {
            let mut config = ShadeConfig::load()?;

            match action {
                CategoryCommands::List => {
                    // Show user-defined categories
                    let user_cats = config.list_categories();
                    if user_cats.is_empty() {
                        println!("No user-defined categories. Using defaults only.");
                        println!();
                        println!("Default categories:");
                        let defaults = default_categories();
                        let mut cat_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
                        for cat in defaults.values() {
                            *cat_counts.entry(cat.as_str()).or_insert(0) += 1;
                        }
                        let mut cats: Vec<_> = cat_counts.into_iter().collect();
                        cats.sort_by_key(|(name, _)| *name);
                        for (name, count) in cats {
                            println!("  {:20} ({} apps)", name, count);
                        }
                    } else {
                        println!("User-defined categories:");
                        for (name, count) in &user_cats {
                            println!("  {:20} ({} apps)", name, count);
                        }
                        println!();
                        println!("Plus {} built-in default categories", {
                            let defaults = default_categories();
                            let mut unique: std::collections::HashSet<&str> = std::collections::HashSet::new();
                            for v in defaults.values() {
                                unique.insert(v.as_str());
                            }
                            unique.len()
                        });
                    }
                }

                CategoryCommands::Add { bundle_id, category } => {
                    config.add_to_category(&bundle_id, &category);
                    config.save()?;
                    println!("Added '{}' to category '{}'", bundle_id, category);
                }

                CategoryCommands::Remove { bundle_id, category } => {
                    if config.remove_from_category(&bundle_id, &category) {
                        config.save()?;
                        println!("Removed '{}' from category '{}'", bundle_id, category);
                    } else {
                        println!("App '{}' not found in category '{}'", bundle_id, category);
                    }
                }

                CategoryCommands::Show { category } => {
                    let user_categories = config.category_map();
                    let all_categories = merge_categories(&user_categories, true);
                    
                    let apps_in_category: Vec<_> = all_categories
                        .iter()
                        .filter(|(_, cat)| cat.as_str() == category)
                        .map(|(bundle_id, _)| bundle_id.as_str())
                        .collect();
                    
                    if apps_in_category.is_empty() {
                        println!("No apps in category '{}'", category);
                    } else {
                        println!("Apps in '{}' ({} total):", category, apps_in_category.len());
                        for bundle_id in apps_in_category {
                            println!("  {}", bundle_id);
                        }
                    }
                }
            }
        }

        Commands::Goals { action } => {
            let mut config = ShadeConfig::load()?;

            match action {
                GoalCommands::List => {
                    let goals = config.list_goals();
                    if goals.is_empty() {
                        println!("No time goals set.");
                        println!();
                        println!("Add a goal with:");
                        println!("  shade goals add <app-bundle-id> <minutes>");
                        println!("  shade goals add <category> <minutes> --category");
                    } else {
                        println!("Time Goals:");
                        println!();
                        for goal in goals {
                            let kind = if goal.is_category { "category" } else { "app" };
                            let hours = goal.daily_limit_minutes / 60;
                            let mins = goal.daily_limit_minutes % 60;
                            if hours > 0 {
                                println!("  {:30} {}h {}m/day ({})", goal.target, hours, mins, kind);
                            } else {
                                println!("  {:30} {}m/day ({})", goal.target, mins, kind);
                            }
                        }
                    }
                }

                GoalCommands::Add { target, limit, category } => {
                    let goal = if category {
                        TimeGoal::for_category(&target, limit)
                    } else {
                        TimeGoal::for_app(&target, limit)
                    };

                    if config.add_goal(goal) {
                        config.save()?;
                        let kind = if category { "category" } else { "app" };
                        println!("Added {} goal: {} ({} min/day)", kind, target, limit);
                    } else {
                        println!("Goal already exists for '{}'", target);
                    }
                }

                GoalCommands::Remove { target, category } => {
                    if config.remove_goal(&target, category) {
                        config.save()?;
                        println!("Removed goal for '{}'", target);
                    } else {
                        println!("No goal found for '{}'", target);
                    }
                }

                GoalCommands::Status => {
                    let goals = config.list_goals();
                    if goals.is_empty() {
                        println!("No time goals set. Use 'shade goals add' to create one.");
                        return Ok(());
                    }

                    if !config.db_path.exists() {
                        println!("No usage data yet. Run 'shade start' to begin tracking.");
                        return Ok(());
                    }

                    let db = Database::open(&config.db_path)?;
                    let today = Utc::now().date_naive();
                    let user_categories = config.category_map();
                    let categories = merge_categories(&user_categories, true);
                    let summary = compute_daily_summary(&db, today, Some(&categories))?;

                    // Build usage maps (convert seconds to minutes)
                    let mut app_usage: HashMap<String, u32> = HashMap::new();
                    let mut category_usage: HashMap<String, u32> = HashMap::new();

                    for app in &summary.top_apps {
                        app_usage.insert(app.bundle_id.clone(), (app.seconds / 60) as u32);
                    }

                    for cat in &summary.category_breakdown {
                        category_usage.insert(cat.category.clone(), (cat.seconds / 60) as u32);
                    }

                    let progress_list = check_goals(&app_usage, &category_usage, goals);

                    println!("Goal Progress (Today):");
                    println!();

                    for progress in &progress_list {
                        let status = match progress.warning_level() {
                            WarningLevel::Exceeded => "OVER LIMIT",
                            WarningLevel::Warning => "WARNING",
                            WarningLevel::Normal => "OK",
                        };
                        
                        let kind = if progress.goal.is_category { "cat" } else { "app" };
                        let limit = progress.goal.daily_limit_minutes;
                        
                        println!(
                            "  {:25} {:>3}m / {:>3}m ({:>5.1}%) [{}] ({})",
                            progress.goal.target,
                            progress.used_minutes,
                            limit,
                            progress.percent_used,
                            status,
                            kind
                        );
                        
                        if progress.warning_level() != WarningLevel::Exceeded {
                            println!("    {}", progress.remaining_display());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
