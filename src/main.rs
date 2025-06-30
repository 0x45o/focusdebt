use clap::{Parser, Subcommand};
use std::process;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::{Utc, Datelike};
use std::path::PathBuf;

mod tracking;
mod storage;
mod stats;
mod utils;
mod config;
mod export;

use tracking::FocusTracker;
use storage::Database;
use stats::Stats;
use utils::{check_dependencies, is_daemon_running, write_pid_file, remove_pid_file, get_current_pid, sleep_ms, ensure_data_directory};
use config::Config;
use export::Exporter;

#[derive(Parser)]
#[command(name = "focusdebt")]
#[command(about = "A CLI tool to track developer focus and context switching")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start background tracking daemon
    Start,
    /// Stop daemon and show session summary
    Stop,
    /// Show daily breakdown with metrics
    Stats {
        /// Show weekly stats instead of daily
        #[arg(long)]
        weekly: bool,
    },
    /// Generate ASCII art report for sharing
    Share,
    /// Export data in various formats
    Export {
        /// Export format (json, csv, html)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Start date (YYYY-MM-DD)
        #[arg(short, long)]
        start_date: Option<String>,
        /// End date (YYYY-MM-DD)
        #[arg(short, long)]
        end_date: Option<String>,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Manage focus applications
    Focusapp {
        #[command(subcommand)]
        action: FocusappCommands,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum FocusappCommands {
    /// Add application to focus list
    Add { name: String },
    /// Remove application from focus list
    Remove { name: String },
    /// List focus vs distraction apps
    List,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Reset configuration to defaults
    Reset,
}

fn main() {
    let cli = Cli::parse();

    // Check dependencies
    if !check_dependencies() {
        eprintln!("❌ Required dependencies not found.");
        #[cfg(target_os = "linux")]
        eprintln!("   Please install xdotool: sudo pacman -S xdotool");
        #[cfg(target_os = "macos")]
        eprintln!("   macOS should work out of the box");
        #[cfg(target_os = "windows")]
        eprintln!("   Windows should work out of the box");
        process::exit(1);
    }

    match cli.command {
        Commands::Start => {
            if is_daemon_running() {
                println!("⚠️  Focus tracking daemon is already running");
                return;
            }

            println!("🚀 Starting focus tracking daemon...");
            start_daemon();
        }
        Commands::Stop => {
            if !is_daemon_running() {
                println!("⚠️  No focus tracking daemon is running");
                return;
            }

            println!("🛑 Stopping daemon and showing session summary...");
            stop_daemon();
        }
        Commands::Stats { weekly } => {
            if weekly {
                println!("📊 Showing weekly focus statistics...");
                show_weekly_stats();
            } else {
                println!("📊 Showing daily focus statistics...");
                show_daily_stats();
            }
        }
        Commands::Share => {
            println!("📤 Generating shareable focus report...");
            generate_share_report();
        }
        Commands::Export { format, start_date, end_date, output } => {
            println!("📤 Exporting data in {} format...", format);
            export_data(format, start_date, end_date, output);
        }
        Commands::Focusapp { action } => match action {
            FocusappCommands::Add { name } => {
                println!("➕ Adding '{}' to focus apps list", name);
                add_focus_app(&name);
            }
            FocusappCommands::Remove { name } => {
                println!("➖ Removing '{}' from focus apps list", name);
                remove_focus_app(&name);
            }
            FocusappCommands::List => {
                println!("📋 Listing focus vs distraction apps...");
                list_focus_apps();
            }
        },
        Commands::Config { action } => match action {
            ConfigCommands::Show => {
                println!("⚙️  Showing current configuration...");
                show_config();
            }
            ConfigCommands::Set { key, value } => {
                println!("⚙️  Setting {} = {}", key, value);
                set_config(&key, &value);
            }
            ConfigCommands::Reset => {
                println!("⚙️  Resetting configuration to defaults...");
                reset_config();
            }
        },
    }
}

fn start_daemon() {
    // Load configuration
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("⚠️  Failed to load config, using defaults: {}", e);
            Config::default()
        }
    };

    // Ensure data directory exists
    if let Err(e) = ensure_data_directory() {
        eprintln!("❌ Failed to create data directory: {}", e);
        process::exit(1);
    }

    // Write PID file
    if let Err(e) = write_pid_file(get_current_pid()) {
        eprintln!("❌ Failed to write PID file: {}", e);
        process::exit(1);
    }

    // Initialize database
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            process::exit(1);
        }
    };

    // Load focus apps from config and database
    let mut focus_apps = config.focus_apps.clone();
    if let Ok(db_apps) = db.get_focus_apps() {
        for app in db_apps {
            if !focus_apps.contains(&app) {
                focus_apps.push(app);
            }
        }
    }

    // Create shared tracker
    let tracker = Arc::new(Mutex::new(FocusTracker::new()));
    
    // Add focus apps to tracker
    {
        let mut tracker = tracker.lock().unwrap();
        for app in focus_apps {
            tracker.add_focus_app(app);
        }
        tracker.start_tracking();
    }

    // Create shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone1 = Arc::clone(&shutdown);
    let shutdown_clone2 = Arc::clone(&shutdown);

    // Create channel for communication
    let (tx, rx) = mpsc::channel();
    let tracker_clone1 = Arc::clone(&tracker);
    let tracker_clone2 = Arc::clone(&tracker);
    let db_clone = db;

    // Spawn tracking thread
    let tracking_thread = thread::spawn(move || {
        let mut last_window = None;
        
        while !shutdown_clone1.load(Ordering::Relaxed) {
            // Check for stop signal
            if let Ok(_) = rx.try_recv() {
                break;
            }

            // Get active window using platform-specific code
            if let Some((app_name, window_title)) = tracking::platform::get_active_window() {
                let current_window = (app_name.clone(), window_title.clone());
                
                if last_window.as_ref() != Some(&current_window) {
                    let mut tracker = tracker_clone1.lock().unwrap();
                    tracker.update_active_window(app_name, window_title);
                    last_window = Some(current_window);
                }
            }

            sleep_ms(config.tracking_interval_ms); // Use config interval
        }
    });

    // Spawn save thread with proper shutdown
    let save_thread = thread::spawn(move || {
        while !shutdown_clone2.load(Ordering::Relaxed) {
            sleep_ms(config.save_interval_ms); // Use config interval
            
            // Check shutdown again before processing
            if shutdown_clone2.load(Ordering::Relaxed) {
                break;
            }
            
            let tracker = tracker_clone2.lock().unwrap();
            if let Some(session) = tracker.get_current_session() {
                if let Err(e) = db_clone.save_focus_session(session) {
                    eprintln!("⚠️  Failed to save session: {}", e);
                }
            }
            
            for switch in tracker.get_context_switches() {
                if let Err(e) = db_clone.save_context_switch(switch) {
                    eprintln!("⚠️  Failed to save context switch: {}", e);
                }
            }
        }
    });

    println!("✅ Focus tracking daemon started successfully");
    println!("📊 Tracking active windows and context switches...");
    println!("💡 Use 'focusdebt stop' to stop tracking and view summary");

    // Wait for stop signal
    loop {
        sleep_ms(1000);
        if !is_daemon_running() {
            break;
        }
    }

    // Signal shutdown
    shutdown.store(true, Ordering::Relaxed);
    
    // Send stop signal to tracking thread
    let _ = tx.send(());
    
    // Wait for threads to finish with timeout
    let tracking_handle = tracking_thread.join();
    let save_handle = save_thread.join();
    
    match tracking_handle {
        Ok(_) => println!("✅ Tracking thread stopped gracefully"),
        Err(_) => eprintln!("⚠️  Tracking thread did not stop gracefully"),
    }
    
    match save_handle {
        Ok(_) => println!("✅ Save thread stopped gracefully"),
        Err(_) => eprintln!("⚠️  Save thread did not stop gracefully"),
    }

    // Clean up
    let _ = remove_pid_file();
}

fn stop_daemon() {
    // Remove PID file to signal stop
    if let Err(e) = remove_pid_file() {
        eprintln!("⚠️  Failed to remove PID file: {}", e);
    }

    // Wait a moment for daemon to stop
    sleep_ms(2000);

    // Show session summary
    show_session_summary();
}

fn show_session_summary() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    let today = Utc::now();
    match Stats::calculate_daily_stats(&db, today) {
        Ok(stats) => Stats::display_daily_stats(&stats),
        Err(e) => eprintln!("❌ Failed to calculate stats: {}", e),
    }
}

fn show_daily_stats() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    let today = Utc::now();
    match Stats::calculate_daily_stats(&db, today) {
        Ok(stats) => Stats::display_daily_stats(&stats),
        Err(e) => eprintln!("❌ Failed to calculate stats: {}", e),
    }
}

fn show_weekly_stats() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    // Get the start of the current week (Monday)
    let today = Utc::now();
    let days_since_monday = today.weekday().num_days_from_monday();
    let week_start = today - chrono::Duration::days(days_since_monday as i64);

    match Stats::calculate_weekly_stats(&db, week_start) {
        Ok(stats) => Stats::display_weekly_stats(&stats),
        Err(e) => eprintln!("❌ Failed to calculate weekly stats: {}", e),
    }
}

fn generate_share_report() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    let today = Utc::now();
    match Stats::calculate_daily_stats(&db, today) {
        Ok(stats) => {
            let report = Stats::generate_ascii_report(&stats);
            println!("{}", report);
        }
        Err(e) => eprintln!("❌ Failed to generate report: {}", e),
    }
}

fn export_data(format: String, start_date: Option<String>, end_date: Option<String>, output: Option<PathBuf>) {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    // Parse dates
    let start = if let Some(date_str) = start_date {
        match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(date) => date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            Err(_) => {
                eprintln!("❌ Invalid start date format. Use YYYY-MM-DD");
                return;
            }
        }
    } else {
        // Default to start of current day
        Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
    };

    let end = if let Some(date_str) = end_date {
        match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(date) => date.and_hms_opt(23, 59, 59).unwrap().and_utc(),
            Err(_) => {
                eprintln!("❌ Invalid end date format. Use YYYY-MM-DD");
                return;
            }
        }
    } else {
        // Default to end of current day
        Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc()
    };

    match Exporter::export_data(&db, start, end, &format, output) {
        Ok(_) => println!("✅ Export completed successfully"),
        Err(e) => eprintln!("❌ Export failed: {}", e),
    }
}

fn add_focus_app(app_name: &str) {
    let mut config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            return;
        }
    };

    config.add_focus_app(app_name.to_string());
    
    if let Err(e) = config.save() {
        eprintln!("❌ Failed to save config: {}", e);
        return;
    }

    // Also add to database for backward compatibility
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("⚠️  Failed to initialize database: {}", e);
            return;
        }
    };

    if let Err(e) = db.add_focus_app(app_name) {
        eprintln!("⚠️  Failed to add to database: {}", e);
    }

    println!("✅ Added '{}' to focus apps", app_name);
}

fn remove_focus_app(app_name: &str) {
    let mut config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            return;
        }
    };

    config.remove_focus_app(app_name);
    
    if let Err(e) = config.save() {
        eprintln!("❌ Failed to save config: {}", e);
        return;
    }

    // Also remove from database for backward compatibility
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("⚠️  Failed to initialize database: {}", e);
            return;
        }
    };

    if let Err(e) = db.remove_focus_app(app_name) {
        eprintln!("⚠️  Failed to remove from database: {}", e);
    }

    println!("✅ Removed '{}' from focus apps", app_name);
}

fn list_focus_apps() {
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            return;
        }
    };

    let apps = config.focus_apps;
    if apps.is_empty() {
        println!("📋 No focus apps configured");
        println!("💡 Use 'focusdebt focusapp add <app_name>' to add apps");
    } else {
        println!("📋 Focus Apps:");
        for (i, app) in apps.iter().enumerate() {
            println!("  {}. {}", i + 1, app);
        }
    }
}

fn show_config() {
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            return;
        }
    };

    println!("⚙️  Current Configuration:");
    println!("  Tracking Interval: {}ms", config.tracking_interval_ms);
    println!("  Save Interval: {}ms", config.save_interval_ms);
    println!("  Deep Focus Threshold: {} minutes", config.deep_focus_threshold_minutes);
    println!("  Log Level: {}", config.log_level);
    println!("  Notifications Enabled: {}", config.notifications.enabled);
    println!("  Auto Export: {}", config.export.auto_export);
    println!("  Export Format: {}", config.export.format);
    
    if !config.focus_apps.is_empty() {
        println!("  Focus Apps: {}", config.focus_apps.join(", "));
    }
    
    if !config.ignored_apps.is_empty() {
        println!("  Ignored Apps: {}", config.ignored_apps.join(", "));
    }
}

fn set_config(key: &str, value: &str) {
    let mut config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            return;
        }
    };

    match key {
        "tracking_interval_ms" => {
            if let Ok(val) = value.parse::<u64>() {
                config.tracking_interval_ms = val;
            } else {
                eprintln!("❌ Invalid value for tracking_interval_ms. Must be a number.");
                return;
            }
        }
        "save_interval_ms" => {
            if let Ok(val) = value.parse::<u64>() {
                config.save_interval_ms = val;
            } else {
                eprintln!("❌ Invalid value for save_interval_ms. Must be a number.");
                return;
            }
        }
        "deep_focus_threshold_minutes" => {
            if let Ok(val) = value.parse::<u64>() {
                config.deep_focus_threshold_minutes = val;
            } else {
                eprintln!("❌ Invalid value for deep_focus_threshold_minutes. Must be a number.");
                return;
            }
        }
        "log_level" => {
            config.log_level = value.to_string();
        }
        "notifications.enabled" => {
            config.notifications.enabled = value.parse().unwrap_or(false);
        }
        "export.auto_export" => {
            config.export.auto_export = value.parse().unwrap_or(false);
        }
        "export.format" => {
            config.export.format = value.to_string();
        }
        _ => {
            eprintln!("❌ Unknown configuration key: {}", key);
            return;
        }
    }

    if let Err(e) = config.save() {
        eprintln!("❌ Failed to save config: {}", e);
        return;
    }

    println!("✅ Configuration updated successfully");
}

fn reset_config() {
    let config = Config::default();
    
    if let Err(e) = config.save() {
        eprintln!("❌ Failed to save config: {}", e);
        return;
    }

    println!("✅ Configuration reset to defaults");
}
