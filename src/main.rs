use clap::{Parser, Subcommand};
use std::process;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::Utc;
use std::io::{self, Write};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

mod tracking;
mod storage;
mod stats;
mod utils;
mod config;

use tracking::FocusTracker;
use storage::Database;
use stats::Stats;
use utils::{check_dependencies, is_daemon_running, write_pid_file, remove_pid_file, sleep_ms, ensure_data_directory};
use config::Config;

#[derive(Debug)]
enum DatabaseCommand {
    SaveSession(tracking::FocusSession),
    SaveContextSwitch(tracking::ContextSwitch),
}

#[derive(Parser)]
#[command(name = "focusdebt")]
#[command(about = "A CLI tool to track focus")]
#[command(version = "0.1.0")]
#[command(disable_help_flag = true)]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(disable_help_flag = true)]
enum Commands {
    /// Start background tracking daemon
    Start,
    /// Stop daemon and show session summary
    Stop,
    /// Check stats for the previous session
    Stats,
    /// Nicer display of stats for sharing
    Share,
    /// Manage focus applications
    Focusapp {
        #[command(subcommand)]
        action: FocusappCommands,
    },
    /// Manage focus websites
    Focussite {
        #[command(subcommand)]
        action: FocussiteCommands,
    },
    /// Manage configuration
    /// 
    /// Examples:
    ///   focusdebt config set tracking_interval_ms 2000
    ///   focusdebt config set save_interval_ms 60000
    ///   focusdebt config set deep_focus_threshold_minutes 45
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Manage sessions
    Sessions {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Debug window detection
    Debug,
    /// Manage database
    Database {
        #[command(subcommand)]
        action: DatabaseCommands,
    },
    /// Show help for all commands
    Help,
}

#[derive(Subcommand)]
#[command(disable_help_flag = true)]
enum FocusappCommands {
    /// Add application to focus list
    Add { name: String },
    /// Remove application from focus list
    Remove { name: String },
    /// List focus apps
    List,
    /// Suggest running GUI applications to add as focus apps
    Suggest,
    /// Show help for focusapp commands
    Help,
}

#[derive(Subcommand)]
#[command(disable_help_flag = true)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key (tracking_interval_ms, save_interval_ms, deep_focus_threshold_minutes)
        key: String,
        /// Configuration value
        value: String,
    },
    /// Reset configuration to defaults
    Reset,
    /// Show help for config commands
    Help,
}

#[derive(Subcommand)]
#[command(disable_help_flag = true)]
enum DatabaseCommands {
    /// Clear all data from database
    Clear,

    /// Clean up invalid sessions
    Cleanup,
    /// Optimize database
    Optimize,
    /// Show help for database commands
    Help,
}

#[derive(Subcommand)]
#[command(disable_help_flag = true)]
enum SessionCommands {
    /// List past sessions
    List,
    /// Show individual session by name
    Show {
        /// Session name
        name: String,
    },
    /// Show help for session commands
    Help,
}

#[derive(Subcommand)]
#[command(disable_help_flag = true)]
enum FocussiteCommands {
    /// Add website to focus list (tracked by tab names)
    Add { domain: String },
    /// Remove website from focus list
    Remove { domain: String },
    /// List focus vs distraction sites
    List,
    /// Suggest currently open browser tabs to add as focus sites
    Suggest,
    /// Show help for focussite commands
    Help,
}

fn main() {
    let cli = Cli::parse();

    // Check for first run and show welcome message
    if let Ok(mut config) = Config::load() {
        if config.first_run {
            show_welcome_message();
            if let Err(e) = config.mark_first_run_complete() {
                eprintln!("❌ Failed to save first run status: {}", e);
            }
        }
    }

    // Check dependencies
    if !check_dependencies() {
        eprintln!("~=~ Required dependencies not found.");
        #[cfg(target_os = "linux")]
        eprintln!("~=~ Please install xdotool: sudo pacman -S xdotool");
        #[cfg(target_os = "macos")]
        eprintln!("~=~ macOS should work out of the box");
        #[cfg(target_os = "windows")]
        eprintln!("~=~ Windows should work out of the box");
        process::exit(1);
    }

    match cli.command {
        Commands::Start => {
            if is_daemon_running() {
                println!("~=~ Focus tracking daemon is already running");
                return;
            }

            println!("~=~ Starting focus tracking daemon...");
            start_daemon();
        }
        Commands::Stop => {
            if !is_daemon_running() {
                println!("~=~ No focus tracking daemon is running");
                return;
            }

            println!("~=~ Stopping daemon and showing session summary...");
            stop_daemon();
        }
        Commands::Stats => {
            println!("~=~ Showing daily focus statistics...");
            show_daily_stats();
        }
        Commands::Share => {
            println!("~=~ Generating shareable focus report...");
            generate_share_report();
        }
        Commands::Focusapp { action } => match action {
            FocusappCommands::Add { name } => {
                println!("~=~ Adding '{}' to focus apps list (fuzzy match)...", name);
                add_focus_app_fuzzy(&name);
            }
            FocusappCommands::Remove { name } => {
                println!("~=~ Removing '{}' from focus apps list", name);
                remove_focus_app(&name);
            }
            FocusappCommands::List => {
                println!("~=~ Listing focus apps...");
                list_focus_apps();
            }
            FocusappCommands::Suggest => {
                println!("~=~ Suggesting running GUI applications...");
                suggest_focus_apps();
            }
            FocusappCommands::Help => {
                println!("~=~ Showing help for focusapp commands...");
                show_focusapp_help();
            }
        },
        Commands::Focussite { action } => match action {
            FocussiteCommands::Add { domain } => {
                println!("~=~ Adding '{}' to focus sites (fuzzy match)...", domain);
                add_focus_site_fuzzy(&domain);
            }
            FocussiteCommands::Remove { domain } => {
                println!("~=~ Removing '{}' from focus sites", domain);
                remove_focus_site(&domain);
            }
            FocussiteCommands::List => {
                println!("~=~ Listing focus vs distraction sites...");
                list_focus_sites();
            }
            FocussiteCommands::Suggest => {
                println!("~=~ Suggesting currently open browser tabs...");
                suggest_focus_sites();
            }
            FocussiteCommands::Help => {
                println!("~=~ Showing help for focussite commands...");
                show_focussite_help();
            }
        },
        Commands::Config { action } => match action {
            ConfigCommands::Show => {
                println!("~=~ Showing current configuration...");
                show_config();
            }
            ConfigCommands::Set { key, value } => {
                println!("~=~ Setting {} = {}", key, value);
                set_config(&key, &value);
            }
            ConfigCommands::Reset => {
                println!("~=~ Resetting configuration to defaults...");
                reset_config();
            }
            ConfigCommands::Help => {
                println!("~=~ Showing help for config commands...");
                show_config_help();
            }
        },
        Commands::Debug => {
            println!("~=~ Debugging window detection...");
            debug_window_detection();
        },
        Commands::Database { action } => match action {
            DatabaseCommands::Clear => {
                println!("~=~ Clearing all database data...");
                clear_database();
            }

            DatabaseCommands::Cleanup => {
                println!("~=~ Cleaning up invalid sessions...");
                cleanup_database();
            }
            DatabaseCommands::Optimize => {
                println!("~=~ Optimizing database...");
                optimize_database();
            }
            DatabaseCommands::Help => {
                println!("~=~ Showing help for database commands...");
                show_database_help();
            }
        }
        Commands::Sessions { action } => match action {
            SessionCommands::List => {
                println!("~=~ Listing past sessions...");
                list_sessions();
            }
            SessionCommands::Show { name } => {
                println!("~=~ Showing session details for: {}", name);
                show_session_details(&name);
            }
            SessionCommands::Help => {
                println!("~=~ Showing help for session commands...");
                show_session_help();
            }
        }
        Commands::Help => {
            show_main_help();
        }
        _ => {}
    }
}

fn start_daemon() {
    // Interactive session name prompt
    println!("\n~=~ Starting FocusDebt Session Tracker\n");
    println!(
        r#"
      >>><<<>>><<<>>><<<>>> .--<12>--. <<<>>><<<>>><<<>>><<<
     >>><<<>>><<<>>><<<>>> /   \      \ <<<>>><<<>>><<<>>><<<
    >>><<<>>><<<>>><<<>>> |     I--    | <<<>>><<<>>><<<>>><<< 
     >>><<<>>><<<>>><<<>>> \          / <<<>>><<<>>><<<>>><<<
      >>><<<>>><<<>>><<<>>> *--<06>--* <<<>>><<<>>><<<>>><<<

    "#
    );
    
    // Get session name with duplicate checking
    let session_name = loop {
        println!("~=~ Please name this focus session:");   
        print!("~=~ Session name: ");
        io::stdout().flush().unwrap();
        let mut input_name = String::new();
        io::stdin().read_line(&mut input_name).unwrap();
        let input_name = input_name.trim().to_string();
        
        // Check if session name is empty
        if input_name.is_empty() {
            println!("❌ Session name cannot be empty. Please try again.\n");
            continue;
        }
        
        // Check if session name already exists
        if let Ok(db) = Database::new() {
            match db.session_name_exists(&input_name) {
                Ok(exists) => {
                    if exists {
                        println!("❌ Session name '{}' already exists. Please choose a different name.\n", input_name);
                        continue;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to check session name: {}. Proceeding anyway.", e);
                }
            }
        }
        
        break input_name;
    };
    
    println!("~=~ Starting session: \"{}\"", session_name);
    println!("~=~ Tracking active windows and context switches...");
    println!("~=~ Use 'focusdebt stop' to end session and view summary\n");

    // Load configuration
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config, using defaults: {}", e);
            Config::default()
        }
    };

    // Ensure data directory exists
    if let Err(e) = ensure_data_directory() {
        eprintln!("❌ Failed to create data directory: {}", e);
        process::exit(1);
    }

    // Fork and daemonize on Unix systems
    #[cfg(unix)]
    {
        unsafe {
            let pid = libc::fork();
            if pid < 0 {
                eprintln!("❌ Failed to fork daemon process");
                process::exit(1);
            } else if pid > 0 {
                // Parent process - write child PID and exit
                if let Err(e) = write_pid_file(pid as u32) {
                    eprintln!("❌ Failed to write PID file: {}", e);
                    process::exit(1);
                }
                println!("~=~ Focus tracking daemon started successfully (PID: {})", pid);
                println!("~=~ Tracking active windows and context switches...");
                println!("~=~ Use 'focusdebt stop' to stop tracking and view summary");
                process::exit(0);
            }
            // Child process continues here
            
            // Create new session
            if libc::setsid() < 0 {
                eprintln!("❌ Failed to create new session");
                process::exit(1);
            }
            
            // Change to root directory to avoid keeping any directory in use
            if libc::chdir(b"/\0".as_ptr() as *const libc::c_char) < 0 {
                eprintln!("❌ Failed to change directory");
                process::exit(1);
            }
            
            // Redirect stdout/stderr to log file for debugging
            let log_path = std::ffi::CString::new("/tmp/focusdebt_daemon.log").unwrap();
            let log_fd = libc::open(log_path.as_ptr(), libc::O_CREAT | libc::O_WRONLY | libc::O_APPEND, 0o644);
            if log_fd >= 0 {
                libc::dup2(log_fd, 1); // stdout
                libc::dup2(log_fd, 2); // stderr
                libc::close(log_fd);
            }
            
            // Also write to a more visible debug file
            let debug_path = std::ffi::CString::new("/tmp/focusdebt_debug.log").unwrap();
            let debug_fd = libc::open(debug_path.as_ptr(), libc::O_CREAT | libc::O_WRONLY | libc::O_APPEND, 0o644);
            if debug_fd >= 0 {
                // Keep original stdout for immediate visibility
                // libc::dup2(debug_fd, 1); // stdout
                libc::close(debug_fd);
            }
            
            // Close stdin
            libc::close(0);
        }
    }

    // On Windows, just write PID file (no proper daemonization)
    #[cfg(windows)]
    {
        if let Err(e) = write_pid_file(get_current_pid()) {
            eprintln!("❌ Failed to write PID file: {}", e);
            process::exit(1);
        }
        println!("~=~ Focus tracking daemon started successfully");
        println!("~=~ Tracking active windows and context switches...");
        println!("~=~ Use 'focusdebt stop' to stop tracking and view summary");
    }

    // Initialize database (will be created in database thread)
    // The database connection will be created in the database thread to avoid thread safety issues

    // Load focus apps from config only (database apps will be loaded when needed)
    let mut focus_apps = config.focus_apps.clone();
    
    // Add some common development apps for testing if none are configured
    if focus_apps.is_empty() {
        focus_apps = vec![
            "code".to_string(),      // VS Code
            "code-oss".to_string(),  // VS Code OSS
            "vim".to_string(),       // Vim
            "nvim".to_string(),      // Neovim
            "emacs".to_string(),     // Emacs
            "subl".to_string(),      // Sublime Text
            "gedit".to_string(),     // Gedit
            "kate".to_string(),      // Kate
            "firefox".to_string(),   // Firefox (for documentation)
            "chromium".to_string(),  // Chromium (for documentation)
        ];
        println!("~=~ No focus apps configured, using defaults: {:?}", focus_apps);
    }

    // Create shared tracker
    let tracker = Arc::new(Mutex::new(FocusTracker::new()));
    
    // Add focus apps to tracker
    {
        let mut tracker = tracker.lock().unwrap();
        for app in focus_apps {
            tracker.add_focus_app(app);
        }
        // Load focus sites from config too
        let focus_sites = config.focus_sites.clone();
        for site in focus_sites {
            tracker.add_focus_site(site);
        }
        tracker.set_session_name(session_name);
        tracker.start_tracking();
    }

    // Create shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone1 = Arc::clone(&shutdown);
    let shutdown_clone2 = Arc::clone(&shutdown);

    // Create channels for communication
    let (tx, rx) = mpsc::channel();
    let (db_tx_raw, db_rx) = mpsc::channel();
    let db_tx = Arc::new(Mutex::new(db_tx_raw));
    let tracker_clone1 = Arc::clone(&tracker);
    let tracker_clone2 = Arc::clone(&tracker);
    let db_tx_save = Arc::clone(&db_tx);

    // Spawn tracking thread
    let tracking_thread = thread::spawn(move || {
        let mut last_window = None;
        let mut consecutive_failures = 0;
        println!("~=~ Tracking thread started");
        
        while !shutdown_clone1.load(Ordering::Relaxed) {
            // Check for stop signal
            if let Ok(_) = rx.try_recv() {
                println!("~=~ Received stop signal");
                break;
            }

            // Get active window using platform-specific code
            match tracking::platform::get_active_window() {
                Some((app_name, window_title)) => {
                    consecutive_failures = 0; // Reset failure counter
                    let current_window = (app_name.clone(), window_title.clone());
                    
                    // Add debug logging to see what's being detected
                    println!("~=~ RAW DETECTION: {} - {}", app_name, window_title);
                    
                    if last_window.as_ref() != Some(&current_window) {
                        println!("~=~ Window changed to: {} - {}", app_name, window_title);
                        let mut tracker = tracker_clone1.lock().unwrap();
                        tracker.update_active_window(app_name, window_title);
                        last_window = Some(current_window);
                    } else {
                        // Same window, just log occasionally for debugging
                        static mut SAME_WINDOW_COUNT: u32 = 0;
                        unsafe {
                            SAME_WINDOW_COUNT += 1;
                            if SAME_WINDOW_COUNT % 100 == 0 {
                                println!("~=~ Still on: {} - {} ({} checks)", app_name, window_title, SAME_WINDOW_COUNT);
                            }
                        }
                    }
                }
                None => {
                    consecutive_failures += 1;
                    // Log failures more frequently at first, then less often
                    if consecutive_failures <= 10 || consecutive_failures % 50 == 0 {
                        println!("❌ Could not get active window (consecutive failures: {})", consecutive_failures);
                    }
                    
                    // If we've had too many consecutive failures, log more details
                    if consecutive_failures == 5 {
                        println!("~=~ Debugging window detection...");
                        // Try to run xdotool manually to see what's happening
                        if let Ok(output) = std::process::Command::new("xdotool")
                            .args(&["getactivewindow"])
                            .output() {
                            println!("   xdotool getactivewindow status: {}", output.status);
                            println!("   xdotool getactivewindow stdout: '{}'", String::from_utf8_lossy(&output.stdout));
                            println!("   xdotool getactivewindow stderr: '{}'", String::from_utf8_lossy(&output.stderr));
                        }
                    }
                }
            }

            sleep_ms(config.tracking_interval_ms); // Use config interval
        }
        
        println!("~=~ Tracking thread exiting");
    });

    // Spawn database thread
    let db_thread = thread::spawn(move || {
        println!("~=~ Database thread started");
        
        // Create database connection in this thread
        let db = match Database::new() {
            Ok(db) => db,
            Err(e) => {
                eprintln!("❌ Failed to initialize database in database thread: {}", e);
                return;
            }
        };
        
        while let Ok(command) = db_rx.recv() {
            match command {
                DatabaseCommand::SaveSession(session) => {
                    if let Err(e) = db.save_focus_session(&session) {
                        eprintln!("❌ Failed to save session: {}", e);
                    } else {
                        println!("~=~ Saved session: {} ({}s)", 
                            session.app_name, 
                            session.duration.as_secs()
                        );
                    }
                }
                DatabaseCommand::SaveContextSwitch(switch) => {
                    if let Err(e) = db.save_context_switch(&switch) {
                        eprintln!("❌ Failed to save context switch: {}", e);
                    } else {
                        println!("~=~ Saved context switch: {} → {}", switch.from_app, switch.to_app);
                    }
                }
            }
        }
        
        println!("~=~ Database thread exiting");
    });

    // Spawn save thread with proper shutdown
    let save_thread = thread::spawn(move || {
        let mut save_counter = 0;
        println!("~=~ Save thread started");
        
        while !shutdown_clone2.load(Ordering::Relaxed) {
            sleep_ms(config.save_interval_ms); // Use config interval
            
            // Check shutdown again before processing
            if shutdown_clone2.load(Ordering::Relaxed) {
                break;
            }
            
            save_counter += 1;
            let mut tracker = tracker_clone2.lock().unwrap();
            
            // Send completed sessions to database thread
            let completed_sessions = tracker.take_completed_sessions();
            for session in completed_sessions {
                if let Err(e) = db_tx_save.lock().unwrap().send(DatabaseCommand::SaveSession(session)) {
                    eprintln!("❌ Failed to send session to database thread: {}", e);
                }
            }
            
            // Send context switches to database thread
            let context_switches = tracker.take_context_switches();
            for switch in context_switches {
                if let Err(e) = db_tx_save.lock().unwrap().send(DatabaseCommand::SaveContextSwitch(switch)) {
                    eprintln!("❌ Failed to send context switch to database thread: {}", e);
                }
            }
            
            // Log stats periodically
            if save_counter % 10 == 0 {
                let stats = tracker.get_stats();
                println!("~=~ Tracker stats: {} sessions, {} switches, current: {}s", 
                    stats.total_sessions, 
                    stats.total_context_switches,
                    stats.current_session_duration.as_secs()
                );
            }
            
            drop(tracker); // Release lock before sleeping
        }
        
        println!("~=~ Save thread exiting");
    });

    // Wait for stop signal
    loop {
        sleep_ms(1000);
        if !is_daemon_running() {
            break;
        }
    }

    // Signal shutdown
    shutdown.store(true, Ordering::Relaxed);
    
    // End current session before stopping
    {
        let mut tracker = tracker.lock().unwrap();
        tracker.end_current_session();
        
        // Send the final session and any remaining data to database thread
        if let Some(session) = tracker.get_current_session() {
            if let Err(e) = db_tx.lock().unwrap().send(DatabaseCommand::SaveSession(session)) {
                eprintln!("❌ Failed to send final session to database thread: {}", e);
            }
        }
        
        // Send any remaining context switches to database thread
        let context_switches = tracker.take_context_switches();
        for switch in context_switches {
            if let Err(e) = db_tx.lock().unwrap().send(DatabaseCommand::SaveContextSwitch(switch)) {
                eprintln!("❌ Failed to send final context switch to database thread: {}", e);
            }
        }
    }
    
    // Send stop signal to tracking thread
    let _ = tx.send(());
    
    // Wait for threads to finish
    let _ = tracking_thread.join();
    let _ = save_thread.join();
    let _ = db_thread.join();

    // Clean up
    let _ = remove_pid_file();
}

fn stop_daemon() {
    // Remove PID file to signal stop
    if let Err(e) = remove_pid_file() {
        eprintln!("❌ Failed to remove PID file: {}", e);
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

    // Get the most recent session name
    let session_name = match db.get_most_recent_session_name() {
        Ok(Some(name)) => name,
        Ok(None) => {
            eprintln!("❌ No recent session found");
            return;
        }
        Err(e) => {
            eprintln!("❌ Failed to get session name: {}", e);
            return;
        }
    };

    // Calculate stats for the specific session
    match Stats::calculate_session_stats(&db, &session_name) {
        Ok(session_stats) => Stats::display_session_summary(&session_stats),
        Err(e) => eprintln!("❌ Failed to calculate session stats: {}", e),
    }
}

fn show_daily_stats() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            return;
        }
    };

    // Calculate stats for today
    let today = chrono::Utc::now();
    match stats::Stats::calculate_daily_stats(&db, today) {
        Ok(daily_stats) => {
            stats::Stats::display_daily_stats(&daily_stats);
        }
        Err(e) => eprintln!("❌ Failed to calculate daily stats: {}", e),
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

    // Get the most recent session name
    let session_name = match db.get_most_recent_session_name() {
        Ok(Some(name)) => name,
        Ok(None) => {
            eprintln!("❌ No recent session found");
            return;
        }
        Err(e) => {
            eprintln!("❌ Failed to get session name: {}", e);
            return;
        }
    };

    // Calculate stats for the specific session
    match Stats::calculate_session_stats(&db, &session_name) {
        Ok(session_stats) => {
            let report = Stats::generate_session_share_report(&session_stats);
            println!("{}", report);
        }
        Err(e) => eprintln!("❌ Failed to generate report: {}", e),
    }
}

fn add_focus_app_fuzzy(input: &str) {
    let mut config = Config::load().unwrap_or_default();
    let running_apps = utils::get_running_apps();
    let matcher = SkimMatcherV2::default();
    let mut best_score = 0;
    let mut best_match = None;
    for (friendly, process) in &running_apps {
        if let Some(score) = matcher.fuzzy_match(friendly, input) {
            if score > best_score {
                best_score = score;
                best_match = Some(process.clone());
            }
        }
        if let Some(score) = matcher.fuzzy_match(process, input) {
            if score > best_score {
                best_score = score;
                best_match = Some(process.clone());
            }
        }
    }
    let app_to_add = best_match.unwrap_or_else(|| input.to_string());
    config.add_focus_app(app_to_add.clone());
    config.save().ok();
    println!("~=~ Added app: {}", app_to_add);
}

fn suggest_focus_apps() {
    let mut running_apps = utils::get_running_apps();
    if running_apps.is_empty() {
        println!("No running GUI applications detected.");
        return;
    }
    // Sort and deduplicate by friendly name
    running_apps.sort_by(|a, b| a.0.cmp(&b.0));
    running_apps.dedup_by(|a, b| a.0 == b.0);
    println!("Currently running GUI applications:");
    for (i, (friendly, process)) in running_apps.iter().enumerate() {
        println!("{}. {} ({})", i + 1, friendly, process);
    }
    println!("Use 'focusdebt focusapp add \"Your App Name\"' to add by fuzzy match.");
}

fn add_focus_site_fuzzy(input: &str) {
    let mut config = Config::load().unwrap_or_default();
    // For demo, just use input as domain (could add fuzzy from recent window titles)
    config.add_focus_site(input.to_lowercase());
    config.save().ok();
    println!("~=~ Added site: {}", input.to_lowercase());
}

fn remove_focus_site(domain: &str) {
    let mut config = Config::load().unwrap_or_default();
    config.remove_focus_site(domain);
    config.save().ok();
    println!("~=~ Removed site: {}", domain);
}

fn list_focus_sites() {
    let config = Config::load().unwrap_or_default();
    if config.focus_sites.is_empty() && config.ignored_sites.is_empty() {
        println!("No focus or distraction sites configured.");
        return;
    }
    println!("Focus Sites:");
    for site in &config.focus_sites {
        println!("  - {}", site);
    }
    println!("Distraction Sites:");
    for site in &config.ignored_sites {
        println!("  - {}", site);
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
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    if let Err(e) = db.add_focus_app(app_name) {
        eprintln!("❌ Failed to add to database: {}", e);
    }

    println!("~=~ Added '{}' to focus apps", app_name);
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
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    if let Err(e) = db.remove_focus_app(app_name) {
        eprintln!("❌ Failed to remove from database: {}", e);
    }

    println!("~=~ Removed '{}' from focus apps", app_name);
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
        println!("~=~ No focus apps configured");
        println!("~=~ Use 'focusdebt focusapp add <app_name>' to add apps");
    } else {
        println!("~=~ Focus Apps:");
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

    println!("~=~ Current Configuration:");
    println!("  Tracking Interval: {}ms", config.tracking_interval_ms);
    println!("  Save Interval: {}ms", config.save_interval_ms);
    println!("  Deep Focus Threshold: {} minutes", config.deep_focus_threshold_minutes);
    
    if !config.focus_apps.is_empty() {
        println!("~=~ Focus Apps: {}", config.focus_apps.join(", "));
    }
    
    if !config.ignored_apps.is_empty() {
        println!("~=~ Ignored Apps: {}", config.ignored_apps.join(", "));
    }
    
    if !config.focus_sites.is_empty() {
        println!("~=~ Focus Sites: {}", config.focus_sites.join(", "));
    }
    
    if !config.ignored_sites.is_empty() {
        println!("~=~ Ignored Sites: {}", config.ignored_sites.join(", "));
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

        _ => {
            eprintln!("❌ Unknown configuration key: {}", key);
            eprintln!("~=~ Available configuration keys:");
            eprintln!("  tracking_interval_ms - How often to check active window (in milliseconds)");
            eprintln!("  save_interval_ms - How often to save data to database (in milliseconds)");
            eprintln!("  deep_focus_threshold_minutes - Minimum duration for deep focus sessions");
            eprintln!("\n~=~ Examples:");
            eprintln!("  focusdebt config set tracking_interval_ms 2000");
            eprintln!("  focusdebt config set save_interval_ms 60000");
            eprintln!("  focusdebt config set deep_focus_threshold_minutes 45");
            return;
        }
    }

    if let Err(e) = config.save() {
        eprintln!("❌ Failed to save config: {}", e);
        return;
    }

    println!("~=~ Configuration updated successfully");
}

fn reset_config() {
    let config = Config::default();
    
    if let Err(e) = config.save() {
        eprintln!("❌ Failed to save config: {}", e);
        return;
    }

    println!("~=~ Configuration reset to defaults");
}

fn debug_window_detection() {
    println!("~=~ Testing window detection...");
    
    // Test multiple times to see if it's working
    for i in 1..=5 {
        println!("\n--- Test {} ---", i);
        match tracking::platform::get_active_window() {
            Some((app_name, window_title)) => {
                println!("~=~ Success: {} - {}", app_name, window_title);
            }
            None => {
                println!("❌ Failed to detect window");
            }
        }
        sleep_ms(1000);
    }
    
    println!("\n~=~ Testing xdotool commands manually...");
    
    // Test xdotool getactivewindow
    match std::process::Command::new("xdotool")
        .args(&["getactivewindow"])
        .output() {
        Ok(output) => {
            println!("xdotool getactivewindow status: {}", output.status);
            println!("xdotool getactivewindow stdout: '{}'", String::from_utf8_lossy(&output.stdout));
            println!("xdotool getactivewindow stderr: '{}'", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("❌ xdotool getactivewindow failed: {}", e);
        }
    }
    
    // Test xdotool getwindowname
    match std::process::Command::new("xdotool")
        .args(&["getactivewindow", "getwindowname"])
        .output() {
        Ok(output) => {
            println!("xdotool getwindowname status: {}", output.status);
            println!("xdotool getwindowname stdout: '{}'", String::from_utf8_lossy(&output.stdout));
            println!("xdotool getwindowname stderr: '{}'", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("❌ xdotool getwindowname failed: {}", e);
        }
    }
    
    // Test xdotool getwindowpid
    match std::process::Command::new("xdotool")
        .args(&["getactivewindow", "getwindowpid"])
        .output() {
        Ok(output) => {
            println!("xdotool getwindowpid status: {}", output.status);
            println!("xdotool getwindowpid stdout: '{}'", String::from_utf8_lossy(&output.stdout));
            println!("xdotool getwindowpid stderr: '{}'", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("❌ xdotool getwindowpid failed: {}", e);
        }
    }
}

fn clear_database() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    match db.clear_all_data() {
        Ok(_) => println!("~=~ Database cleared successfully"),
        Err(e) => eprintln!("❌ Failed to clear database: {}", e),
    }
}

fn cleanup_database() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    match db.clear_invalid_sessions() {
        Ok(deleted) => println!("~=~ Cleaned up {} invalid sessions", deleted),
        Err(e) => eprintln!("❌ Failed to cleanup database: {}", e),
    }
}

fn optimize_database() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    match db.vacuum_database() {
        Ok(_) => println!("~=~ Database optimized successfully"),
        Err(e) => eprintln!("❌ Failed to optimize database: {}", e),
    }
}

fn list_sessions() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    match Stats::list_sessions(&db, None, None) {
        Ok(sessions) => {
            println!("~=~ Sessions:");
            for session in sessions {
                println!("  {}", session);
            }
        }
        Err(e) => eprintln!("❌ Failed to list sessions: {}", e),
    }
}

fn show_session_details(query: &str) {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    match Stats::show_session_details(&db, query) {
        Ok(session) => println!("{}", session),
        Err(e) => eprintln!("❌ Failed to show session details: {}", e),
    }
}

fn show_welcome_message() {
    println!(r#"
Welcome to FocusDebt - CLI Focus Tracker!

FocusDebt helps developers track focus time and context switching.

Focus Apps:
  focusdebt focusapp add code    # Add VS Code as focus app
  focusdebt focusapp list        # View focus vs distraction apps

Configuration:
  focusdebt config show    # View current settings

Get help anytime with --help on any command.
Happy focusing! 🚀
"#);
}

fn suggest_focus_sites() {
    let open_tabs = utils::get_open_browser_tabs();
    if open_tabs.is_empty() {
        println!("No open browser tabs detected.");
        println!("Make sure you have browser windows open with tabs.");
        return;
    }
    
    println!("Currently open browser tab:");
    for (i, tab) in open_tabs.iter().enumerate() {
        println!("{}. {}", i + 1, tab);
    }
    println!("\nUse 'focusdebt focussite add \"Tab Name\"' to add by fuzzy match.");
    println!("Example: focusdebt focussite add \"ChatGPT\"");
}

fn show_focusapp_help() {
    println!("~=~ FocusApp Commands:");
    println!("  add <app_name>     - Add an application to the focus list");
    println!("  remove <app_name>  - Remove an application from the focus list");
    println!("  list               - List all focus applications");
    println!("  suggest            - Suggest running GUI applications");
    println!("  help               - Show this help message");
    println!();
    println!("Examples:");
    println!("  focusdebt focusapp add code");
    println!("  focusdebt focusapp remove firefox");
    println!("  focusdebt focusapp list");
}

fn show_focussite_help() {
    println!("~=~ Focussite Commands:");
    println!("  add <domain>       - Add a website to the focus list");
    println!("  remove <domain>    - Remove a website from the focus list");
    println!("  list               - List all focus websites");
    println!("  suggest            - Suggest currently open browser tabs");
    println!("  help               - Show this help message");
    println!();
    println!("Examples:");
    println!("  focusdebt focussite add github.com");
    println!("  focusdebt focussite remove youtube.com");
    println!("  focusdebt focussite list");
}

fn show_config_help() {
    println!("~=~ Config Commands:");
    println!("  show               - Show current configuration");
    println!("  set <key> <value>  - Set a configuration value");
    println!("  reset              - Reset configuration to defaults");
    println!("  help               - Show this help message");
    println!();
    println!("Available configuration keys:");
    println!("  tracking_interval_ms           - How often to check active window (ms)");
    println!("  save_interval_ms               - How often to save data to database (ms)");
    println!("  deep_focus_threshold_minutes   - Minimum duration for deep focus sessions");
    println!();
    println!("Examples:");
    println!("  focusdebt config set tracking_interval_ms 2000");
    println!("  focusdebt config set save_interval_ms 60000");
    println!("  focusdebt config set deep_focus_threshold_minutes 45");
}

fn show_database_help() {
    println!("~=~ Database Commands:");
    println!("  clear              - Clear all data from the database");
    println!("  cleanup            - Clean up invalid sessions");
    println!("  optimize           - Optimize the database");
    println!("  help               - Show this help message");
    println!();
    println!("Examples:");
    println!("  focusdebt database clear");
    println!("  focusdebt database cleanup");
    println!("  focusdebt database optimize");
}

fn show_session_help() {
    println!("~=~ Session Commands:");
    println!("  list               - List all sessions");
    println!("  show <session_name> - Show details for a specific session");
    println!("  help               - Show this help message");
    println!();
    println!("Examples:");
    println!("  focusdebt sessions list");
    println!("  focusdebt sessions show \"Morning Coding Session\"");
}

fn show_main_help() {
    println!("~=~ FocusDebt - CLI Focus Tracker");
    println!("~=~ A CLI tool to track focus time and context switching");
    println!();
    println!("~=~ Main Commands:");
    println!("  start              - Start background tracking daemon");
    println!("  stop               - Stop daemon and show session summary");
    println!("  stats              - Check stats for the previous session");
    println!("  share              - Nicer display of stats for sharing");
    println!("  debug              - Debug window detection");
    println!("  help               - Show this help message");
    println!();
    println!("~=~ Management Commands:");
    println!("  focusapp <action>  - Manage focus applications");
    println!("  focussite <action> - Manage focus websites");
    println!("  config <action>    - Manage configuration");
    println!("  sessions <action>  - Manage sessions");
    println!("  database <action>  - Manage database");
    println!();
    println!("~=~ Focus Apps:");
    println!("  focusdebt focusapp add code    # Add VS Code as focus app");
    println!("  focusdebt focusapp list        # View focus vs distraction apps");
    println!();
    println!("~=~ Configuration:");
    println!("  focusdebt config show    # View current settings");
    println!("  focusdebt config help    # Show config command help");
    println!();
    println!("~=~ Get help for specific commands:");
    println!("  focusdebt focusapp help  # Focus app management help");
    println!("  focusdebt focussite help # Focus site management help");
    println!("  focusdebt config help    # Configuration help");
    println!("  focusdebt sessions help  # Session management help");
    println!("  focusdebt database help  # Database management help");
    println!();
    println!("Happy focusing! 🚀");
}

