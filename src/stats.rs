use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use crate::tracking::FocusSession;
use crate::storage::Database;
use crate::utils;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct DailyStats {
    pub date: DateTime<Utc>,
    pub total_focus_time: Duration,
    pub total_distraction_time: Duration,
    pub context_switches: usize,
    pub deep_focus_sessions: usize,
    pub focus_efficiency: f64,
    pub most_used_apps: Vec<(String, Duration)>,
    pub most_distracting_apps: Vec<(String, Duration)>,
}

pub struct Stats;

#[derive(Debug, Clone)]
pub struct AggregatedSession {
    pub session_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_duration: Duration,
    pub focus_efficiency: f64,
    pub app_usage: Vec<(String, Duration, bool)>, // (app_name, duration, is_focus)
    pub domain_usage: Vec<(String, Duration, bool)>, // (tab_name, duration, is_focus)
    pub context_switches: usize,
}

impl Stats {
    pub fn calculate_daily_stats(db: &Database, date: DateTime<Utc>) -> Result<DailyStats, Box<dyn std::error::Error>> {
        let sessions = db.get_sessions_for_date(date)?;
        let switches = db.get_context_switches_for_date(date)?;
        let deep_sessions = db.get_deep_focus_sessions(30 * 60, date)?; // 30 minutes
        let most_distracting = db.get_most_distracting_apps(date, 5)?;

        let mut total_focus_time = Duration::ZERO;
        let mut total_distraction_time = Duration::ZERO;
        let mut app_usage: HashMap<String, Duration> = HashMap::new();

        // Process sessions with better validation
        for session in &sessions {
            // Skip sessions with invalid durations (likely from old broken tracking)
            if session.duration > Duration::from_secs(24 * 60 * 60) {
                // Skip sessions longer than 24 hours (likely broken data)
                continue;
            }
            
            if session.duration < Duration::from_secs(1) {
                // Skip sessions shorter than 1 second (likely noise)
                continue;
            }

            if session.is_focus_app {
                total_focus_time += session.duration;
            } else {
                total_distraction_time += session.duration;
            }

            *app_usage.entry(session.app_name.clone()).or_insert(Duration::ZERO) += session.duration;
        }

        let total_time = total_focus_time + total_distraction_time;
        let focus_efficiency = if total_time > Duration::ZERO {
            total_focus_time.as_secs_f64() / total_time.as_secs_f64() * 100.0
        } else {
            0.0
        };

        // Sort apps by usage time and filter out minimal usage
        let mut app_usage_vec: Vec<(String, Duration)> = app_usage.into_iter().collect();
        app_usage_vec.sort_by(|a, b| b.1.cmp(&a.1));

        // Filter out apps with less than 10 seconds of usage
        let min_usage_threshold = Duration::from_secs(10);
        let most_used_apps = app_usage_vec.iter()
            .filter(|(_, duration)| *duration >= min_usage_threshold)
            .take(5)
            .cloned()
            .collect();

        // Filter distracting apps to only show meaningful usage
        let most_distracting_filtered = most_distracting.iter()
            .filter(|(_, duration)| *duration >= min_usage_threshold)
            .cloned()
            .collect();

        Ok(DailyStats {
            date,
            total_focus_time,
            total_distraction_time,
            context_switches: switches.len(),
            deep_focus_sessions: deep_sessions.len(),
            focus_efficiency,
            most_used_apps,
            most_distracting_apps: most_distracting_filtered,
        })
    }

    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    pub fn display_daily_stats(stats: &DailyStats) {
        let today = Utc::now();
        let top_sep = "~~+~~+*+~~+~~+*+~~+~~";
        println!("\n{}\n", top_sep);
        println!("DAILY FOCUS SUMMARY\n");
        println!("{}\n", utils::format_datetime_local(today));
        println!("Focus Time        : {:<30}\n", Self::format_duration(stats.total_focus_time));
        println!("Distraction Time  : {:<30}\n", Self::format_duration(stats.total_distraction_time));
        println!("Context Switches  : {:<30}\n", stats.context_switches);
        println!("Focus Efficiency  : {:<30}\n", {
            let bar_width = 30;
            let filled = ((stats.focus_efficiency / 100.0) * bar_width as f64) as usize;
            let empty = bar_width - filled;
            format!("[{}{}] {:.0}%", "■".repeat(filled), "□".repeat(empty), stats.focus_efficiency)
        });
        if !stats.most_used_apps.is_empty() {
            println!("TOP APPLICATIONS\n");
            for (i, (app, duration)) in stats.most_used_apps.iter().take(5).enumerate() {
                let app_display = if app.len() > 20 { format!("{}...", &app[..17]) } else { app.clone() };
                println!("{}. {:<20} : {:<30}\n", i + 1, app_display, Self::format_duration(*duration));
            }
        }
        println!("{}\n", top_sep);
    }

    pub fn generate_ascii_report(stats: &DailyStats) -> String {
        let mut report = String::new();
        let top_sep = "~~+~~+*+~~+~~+*+~~+~~";
        report.push_str("\n");
        report.push_str(&format!("{}\n\n", top_sep));
        report.push_str(&format!("~=~ FOCUSDEBT REPORT ~=~\n\n"));
        report.push_str(&format!("{}\n\n", utils::format_datetime_local(stats.date)));
        report.push_str(&format!("Focus Time      : {:<42}\n\n", Self::format_duration(stats.total_focus_time)));
        report.push_str(&format!("Distraction     : {:<42}\n\n", Self::format_duration(stats.total_distraction_time)));
        report.push_str(&format!("Context Switches: {:<42}\n\n", stats.context_switches));
        report.push_str(&format!("Deep Sessions   : {:<42}\n\n", stats.deep_focus_sessions));
        report.push_str(&format!("Focus Efficiency\n\n"));
        let efficiency_bar_len = 30;
        let efficiency_filled = ((stats.focus_efficiency / 100.0) * efficiency_bar_len as f64) as usize;
        let efficiency_bar = format!("[{}{}] {:.0}%", 
            "▓".repeat(efficiency_filled), 
            "░".repeat(efficiency_bar_len - efficiency_filled),
            stats.focus_efficiency);
        report.push_str(&format!("   {:<58}\n\n", efficiency_bar));
        if !stats.most_used_apps.is_empty() {
            report.push_str("TOP APPLICATIONS\n\n");
            let max_duration = stats.most_used_apps.first().map(|(_, d)| d.as_secs()).unwrap_or(1);
            for (i, (app, duration)) in stats.most_used_apps.iter().take(4).enumerate() {
                let app_display = if app.len() > 15 { format!("{}...", &app[..12]) } else { app.clone() };
                let duration_str = Self::format_duration(*duration);
                let bar_len = 20;
                let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                let app_bar = format!("[{}{}]", "▓".repeat(filled), "░".repeat(bar_len - filled));
                report.push_str(&format!("{}. {:<15} {} {:<12}\n\n", i + 1, app_display, app_bar, duration_str));
            }
        } else {
            report.push_str("~=~ No significant application usage detected\n\n");
        }
        report.push_str(&format!("{}\n\n", top_sep));
        report
    }

    pub fn generate_session_share_report(session: &AggregatedSession) -> String {
        let mut report = String::new();
        let top_sep = "~~+~~+*+~~+~~+*+~~+~~";
        let start = utils::format_datetime_local(session.start_time);
        let time_range = if let Some(end) = session.end_time {
            format!("{} → {}", 
                utils::format_timestamp_local(session.start_time), 
                utils::format_timestamp_local(end))
        } else {
            format!("{} → ongoing", utils::format_timestamp_local(session.start_time))
        };
        
        // Calculate focus time from app usage
        let focus_time: std::time::Duration = session.app_usage.iter()
            .filter(|(_, _, is_focus)| *is_focus)
            .map(|(_, duration, _)| *duration)
            .sum();
        
        report.push_str("\n");
        report.push_str(&format!("{}\n\n", top_sep));
        report.push_str(&format!("~=~ SESSION REPORT ~=~\n\n"));
                println!(
r#"
----------          -^-       
----------         / * \     
----------        / < > \    
----------       / _-_-_ \   
----------      /=-     -=\  
----------     -     I     -  
----------    -      L /    -  
----------   -L-     o/    -I- 
----------      =_       _=  
----------        =__i__=    
----------         ---|     
----------          | |     
----------          | |     
----------          | |     
----------          o |     
----------         -O-|     
----------          i |     
----------           =I=    
----------          --O--   
----------           -i-    
----------            o
"#
);
        report.push_str(&format!("Session: {}\n\n", session.session_name));
        report.push_str(&format!("Time: {}\n\n", time_range));
        report.push_str(&format!("Focus Time: {}\n\n", Self::format_duration(focus_time)));
        report.push_str(&format!("Focus Efficiency: {:.0}%\n\n", session.focus_efficiency));
        
        // Separate browser apps from regular apps
        let (browser_apps, regular_apps): (Vec<_>, Vec<_>) = session.app_usage.iter()
            .partition(|(app, _, _)| Self::is_browser_app(app));
        
        // Show regular applications (non-browser)
        if !regular_apps.is_empty() {
            report.push_str("TOP APPLICATIONS\n\n");
            let max_duration = regular_apps.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
            for (i, (app, duration, _)) in regular_apps.iter().take(5).enumerate() {
                let app_display = if app.len() > 15 { format!("{}...", &app[..12]) } else { app.clone() };
                let duration_str = Self::format_duration(*duration);
                let bar_len = 20;
                let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                let app_bar = format!("[{}{}]", "▓".repeat(filled), "░".repeat(bar_len - filled));
                report.push_str(&format!("{}. {:<15} {} {:<12}\n\n", i + 1, app_display, app_bar, duration_str));
            }
        }
        
        // Show browser apps grouped together (no individual tabs)
        if !browser_apps.is_empty() {
            report.push_str("BROWSER APPLICATIONS\n\n");
            let max_duration = browser_apps.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
            for (i, (app, duration, _)) in browser_apps.iter().take(5).enumerate() {
                let app_display = if app.len() > 15 { format!("{}...", &app[..12]) } else { app.clone() };
                let duration_str = Self::format_duration(*duration);
                let bar_len = 20;
                let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                let app_bar = format!("[{}{}]", "▓".repeat(filled), "░".repeat(bar_len - filled));
                report.push_str(&format!("{}. {:<15} {} {:<12}\n\n", i + 1, app_display, app_bar, duration_str));
            }
        }
        report.push_str(&format!("{}\n\n", top_sep));
        report
    }

    pub fn list_sessions(db: &Database, _last: Option<usize>, _date: Option<String>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut sessions = Vec::new();
        
        // Get all sessions from last 30 days
        let mut all_sessions = Vec::new();
        for days_ago in 0..30 {
            let dt = Utc::now() - chrono::Duration::days(days_ago);
            let day_sessions = db.get_sessions_for_date(dt)?;
            all_sessions.extend(day_sessions);
        }
        
        // Group by session name and aggregate
        let aggregated = Self::aggregate_sessions_by_name(&all_sessions);
        let take_n = 20; // Show last 20 sessions
        for (i, session) in aggregated.iter().take(take_n).enumerate() {
            sessions.push(Self::format_session_summary(i + 1, session));
        }
        Ok(sessions)
    }

    pub fn show_session_details(db: &Database, query: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Get all sessions from last 30 days
        let mut all_sessions = Vec::new();
        for days_ago in 0..30 {
            let dt = Utc::now() - chrono::Duration::days(days_ago);
            let day_sessions = db.get_sessions_for_date(dt)?;
            all_sessions.extend(day_sessions);
        }
        
        // Group by session name
        let aggregated = Self::aggregate_sessions_by_name(&all_sessions);
        
        // Search by session name (case-insensitive)
        for session in aggregated {
            if session.session_name.eq_ignore_ascii_case(query) {
                return Ok(Self::format_session_report(&session));
            }
        }
        
        Err(format!("❌ Session not found: {}", query).into())
    }

    fn aggregate_sessions_by_name(sessions: &[FocusSession]) -> Vec<AggregatedSession> {
        use std::collections::HashMap;
        
        let mut session_groups: HashMap<String, Vec<&FocusSession>> = HashMap::new();
        
        for session in sessions {
            let session_key = if session.session_name.is_empty() {
                // Create unique keys for each app session instead of grouping all as "unnamed"
                format!("{}_{}", session.app_name, session.start_time.timestamp())
            } else {
                session.session_name.clone()
            };
            session_groups.entry(session_key)
                .or_insert_with(Vec::new)
                .push(session);
        }
        
        let mut aggregated = Vec::new();
        for (name, group_sessions) in session_groups {
            if group_sessions.is_empty() { continue; }
            
            let start_time = group_sessions.iter().map(|s| s.start_time).min().unwrap();
            let end_time = group_sessions.iter()
                .filter_map(|s| s.end_time)
                .max();
            
            // Calculate total duration from actual start and end times, not sum of individual durations
            let total_duration = if let Some(end_time) = end_time {
                end_time.signed_duration_since(start_time).to_std().unwrap_or(Duration::ZERO)
            } else {
                // If no end time, sum individual durations as fallback
                group_sessions.iter().map(|s| s.duration).sum()
            };
            
            let focus_time: Duration = group_sessions.iter()
                .filter(|s| s.is_focus_app)
                .map(|s| s.duration)
                .sum();
            
            let focus_efficiency = if total_duration > Duration::ZERO {
                (focus_time.as_secs_f64() / total_duration.as_secs_f64()) * 100.0
            } else {
                0.0
            };
            
            // Collect unique apps with their total usage
            let mut app_usage: HashMap<String, Duration> = HashMap::new();
            let mut domain_usage: HashMap<String, Duration> = HashMap::new();
            

            
            for session in &group_sessions {
                *app_usage.entry(session.app_name.clone()).or_insert(Duration::ZERO) += session.duration;
                

                
                // Also collect domain usage if available
                if let Some(domain) = &session.domain {
                    *domain_usage.entry(domain.clone()).or_insert(Duration::ZERO) += session.duration;
                }
            }
            

            
            let mut app_list: Vec<(String, Duration, bool)> = app_usage.into_iter()
                .map(|(app, duration)| {
                    let is_focus = group_sessions.iter()
                        .any(|s| s.app_name == app && s.is_focus_app);
                    (app, duration, is_focus)
                })
                .collect();
            app_list.sort_by(|a, b| b.1.cmp(&a.1));
            
            let mut domain_list: Vec<(String, Duration, bool)> = domain_usage.into_iter()
                .map(|(tab_name, duration)| {
                    // Check if any session with this tab name was marked as focus
                    let is_focus = group_sessions.iter()
                        .any(|s| s.domain.as_ref() == Some(&tab_name) && s.is_focus_app);
                    (tab_name, duration, is_focus)
                })
                .collect();
            domain_list.sort_by(|a, b| b.1.cmp(&a.1));
            
            aggregated.push(AggregatedSession {
                session_name: name,
                start_time,
                end_time,
                total_duration,
                focus_efficiency,
                app_usage: app_list,
                domain_usage: domain_list,
                context_switches: group_sessions.len().saturating_sub(1), // Rough estimate
            });
        }
        
        // Sort by start time (newest first)
        aggregated.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        aggregated
    }

    fn format_session_summary(idx: usize, s: &AggregatedSession) -> String {
        let start = utils::format_datetime_local(s.start_time);
        let end = s.end_time.map(|t| utils::format_timestamp_local(t)).unwrap_or("--".to_string());
        let duration = Self::format_duration(s.total_duration);
        let focus_percent = format!("{:.0}%", s.focus_efficiency);
        
        // Format with proper spacing to match example
        let time_range = format!("{}-{}", start, end);
        format!("{}. \"{}\"        {}  {}  Focus: {}", 
            idx, s.session_name, time_range, duration, focus_percent)
    }

    fn format_session_report(s: &AggregatedSession) -> String {
        let start = utils::format_datetime_local(s.start_time);
        let end = s.end_time.map(|t| utils::format_timestamp_local(t)).unwrap_or("ongoing".to_string());
        let duration = Self::format_duration(s.total_duration);
        let top_sep = "~~+~~+*+~~+~~+*+~~+~~";
        let bar_width = 30;
        let filled = ((s.focus_efficiency / 100.0) * bar_width as f64) as usize;
        let empty = bar_width - filled;
        let efficiency_bar = format!("[{}{}] {:.0}%", 
            "▓".repeat(filled), 
            "░".repeat(empty), 
            s.focus_efficiency);
        let mut report = String::new();
        report.push_str("\n");
        report.push_str(&format!("{}\n\n", top_sep));
        report.push_str("~=~ SESSION DETAILS ~=~\n\n");
        report.push_str(&format!("Name:       {:<48}\n\n", if s.session_name.len() > 48 {
            format!("{}...", &s.session_name[..45])
        } else {
            s.session_name.clone()
        }));
        let time_line = format!("Duration:  {} → {} ({})", start, end, duration);
        report.push_str(&format!("{}\n\n", time_line));
        report.push_str(&format!("Efficiency: {:<48}\n\n", efficiency_bar));
        report.push_str(&format!("Switches:   {:<48}\n\n", s.context_switches));
        // Separate browser apps from regular apps
        let (browser_apps, regular_apps): (Vec<_>, Vec<_>) = s.app_usage.iter()
            .partition(|(app, _, _)| Self::is_browser_app(app));
        
        // Show regular applications (non-browser)
        if !regular_apps.is_empty() {
            report.push_str("~=~ APPLICATION BREAKDOWN ~=~\n\n");
            let max_duration = regular_apps.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
            for (i, (app, duration, is_focus)) in regular_apps.iter().take(6).enumerate() {
                let app_display = if app.len() > 20 { format!("{}...", &app[..17]) } else { app.clone() };
                let duration_str = Self::format_duration(*duration);
                let focus_text = if *is_focus { "Focus" } else { "Other" };
                let bar_len = 20;
                let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                let usage_bar = format!("[{}{}]", "■".repeat(filled), "□".repeat(bar_len - filled));
                let app_line = format!("{:<20} {} {:<10} ({:<5})", app_display, usage_bar, duration_str, focus_text);
                report.push_str(&format!("{}\n\n", app_line));
            }
        }
        
        // Show browser apps grouped together (no individual tabs)
        if !browser_apps.is_empty() {
            report.push_str("~=~ BROWSER APPLICATIONS ~=~\n");
            let max_duration = browser_apps.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
            for (app, duration, is_focus) in browser_apps.iter().take(6) {
                let app_display = if app.len() > 18 { format!("{}...", &app[..15]) } else { app.clone() };
                let duration_str = Self::format_duration(*duration);
                let focus_text = if *is_focus { "Focus" } else { "Other" };
                let bar_len = 15;
                let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                let usage_bar = format!("[{}{}]", "■".repeat(filled), "□".repeat(bar_len - filled));
                let app_line = format!("{:<18} {} {:<8} ({:<5})", app_display, usage_bar, duration_str, focus_text);
                report.push_str(&format!("{}\n\n", app_line));
            }
        }
        
        // Show browser tabs individually (for session details)
        if !s.domain_usage.is_empty() {
            // Group browser tabs by browser name
            let mut browser_tab_map: std::collections::BTreeMap<String, Vec<(String, std::time::Duration, bool)>> = std::collections::BTreeMap::new();
            for (tab_name, duration, is_focus) in &s.domain_usage {
                // Group by browser based on tab name suffix
                let browser = if tab_name.to_lowercase().contains("chrome") {
                    "CHROME TABS"
                } else if tab_name.to_lowercase().contains("brave") {
                    "BRAVE TABS"
                } else if tab_name.to_lowercase().contains("firefox") {
                    "FIREFOX TABS"
                } else if tab_name.to_lowercase().contains("safari") {
                    "SAFARI TABS"
                } else if tab_name.to_lowercase().contains("edge") {
                    "EDGE TABS"
                } else if tab_name.to_lowercase().contains("opera") {
                    "OPERA TABS"
                } else if tab_name.to_lowercase().contains("vivaldi") {
                    "VIVALDI TABS"
                } else {
                    "BROWSER TABS"
                };
                browser_tab_map.entry(browser.to_string()).or_default().push((tab_name.clone(), *duration, *is_focus));
            }
            for (browser, tabs) in browser_tab_map {
                report.push_str(&format!("~=~ {} (TOP 5) ~=~\n\n", browser));
                let max_duration = tabs.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
                for (tab_name, duration, is_focus) in tabs.iter().take(5) {
                    let tab_display = if tab_name.len() > 30 { format!("{}...", &tab_name[..27]) } else { tab_name.clone() };
                    let duration_str = Self::format_duration(*duration);
                    let focus_text = if *is_focus { "Focus" } else { "Other" };
                    let bar_len = 15;
                    let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                    let usage_bar = format!("[{}{}]", "■".repeat(filled), "□".repeat(bar_len - filled));
                    let tab_line = format!("{:<30} {} {:<8} ({:<5})", tab_display, usage_bar, duration_str, focus_text);
                    report.push_str(&format!("{}\n\n", tab_line));
                }
            }
        }
        report.push_str(&format!("{}\n\n", top_sep));
        report
    }

    pub fn calculate_session_stats(db: &Database, session_name: &str) -> Result<AggregatedSession, Box<dyn std::error::Error>> {
        // Get all sessions from last 30 days
        let mut all_sessions = Vec::new();
        for days_ago in 0..30 {
            let dt = Utc::now() - chrono::Duration::days(days_ago);
            let day_sessions = db.get_sessions_for_date(dt)?;
            all_sessions.extend(day_sessions);
        }
        
        // Filter sessions by the specific session name
        let session_sessions: Vec<FocusSession> = all_sessions
            .into_iter()
            .filter(|s| s.session_name.eq_ignore_ascii_case(session_name))
            .collect();
        

        
        if session_sessions.is_empty() {
            return Err(format!("❌ No sessions found with name: {}", session_name).into());
        }
        
        // Aggregate the sessions
        let aggregated = Self::aggregate_sessions_by_name(&session_sessions);
        
        // Return the first (and should be only) aggregated session
        if let Some(session) = aggregated.first() {
            Ok(session.clone())
        } else {
            Err(format!("❌ Failed to aggregate session: {}", session_name).into())
        }
    }

    pub fn display_session_summary(session: &AggregatedSession) {
        let start = utils::format_datetime_local(session.start_time);
        let end = session.end_time.map(|t| utils::format_timestamp_local(t)).unwrap_or("ongoing".to_string());
        let duration = Self::format_duration(session.total_duration);
        let top_sep = "~~+~~+*+~~+~~+*+~~+~~";
        let bar_width = 25;
        let filled = ((session.focus_efficiency / 100.0) * bar_width as f64) as usize;
        let empty = bar_width - filled;
        let efficiency_display = format!("{:.0}% [{}{}]", 
            session.focus_efficiency,
            "▓".repeat(filled), 
            "░".repeat(empty));
        
        println!("\n{}\n", top_sep);
        println!("~=~ SESSION COMPLETE ~=~\n");
        println!(
r#"
----------          -^-       
----------         / * \     
----------        / < > \    
----------       / _-_-_ \   
----------      /=-     -=\  
----------     -     I     -  
----------    -      L /    -  
----------   -L-     o/    -I- 
----------      =_       _=  
----------        =__i__=    
----------         ---|     
----------          | |     
----------          | |     
----------          | |     
----------          o |     
----------         -O-|     
----------          i |     
----------           =I=    
----------          --O--   
----------           -i-    
----------            o
"#
);
        println!("Session: {:<48}\n", if session.session_name.len() > 48 { 
            format!("{}...", &session.session_name[..45]) 
        } else { 
            session.session_name.clone() 
        });
        let time_line = format!("Duration: {} → {} ({})", start, end, duration);
        println!("{}\n", time_line);
        println!("Focus:   {:<48}\n", efficiency_display);
        println!("Switches: {:<47}\n", session.context_switches);
        
        // Separate browser apps from regular apps
        let (browser_apps, regular_apps): (Vec<_>, Vec<_>) = session.app_usage.iter()
            .partition(|(app, _, _)| Self::is_browser_app(app));
        
        // Show regular applications (non-browser)
        if !regular_apps.is_empty() {
            println!("~=~ APPLICATIONS USED ~=~\n");
            let max_duration = regular_apps.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
            for (i, (app, duration, is_focus)) in regular_apps.iter().take(6).enumerate() {
                let app_display = if app.len() > 18 { format!("{}...", &app[..15]) } else { app.clone() };
                let duration_str = Self::format_duration(*duration);
                let focus_text = if *is_focus { "Focus" } else { "Other" };
                let bar_len = 15;
                let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                let usage_bar = format!("[{}{}]", "■".repeat(filled), "□".repeat(bar_len - filled));
                let app_line = format!("{:<18} {} {:<8} ({:<5})", app_display, usage_bar, duration_str, focus_text);
                println!("{}\n", app_line);
            }
        }
        

        
        // Show browser tabs individually (for session summary)
        if !session.domain_usage.is_empty() {
            // Group browser tabs by browser name
            let mut browser_tab_map: BTreeMap<String, Vec<(String, std::time::Duration, bool)>> = BTreeMap::new();
            for (tab_name, duration, is_focus) in &session.domain_usage {
                // Group by browser based on tab name suffix
                let browser = if tab_name.to_lowercase().contains("chrome") {
                    "CHROME TABS"
                } else if tab_name.to_lowercase().contains("brave") {
                    "BRAVE TABS"
                } else if tab_name.to_lowercase().contains("firefox") {
                    "FIREFOX TABS"
                } else if tab_name.to_lowercase().contains("safari") {
                    "SAFARI TABS"
                } else if tab_name.to_lowercase().contains("edge") {
                    "EDGE TABS"
                } else if tab_name.to_lowercase().contains("opera") {
                    "OPERA TABS"
                } else if tab_name.to_lowercase().contains("vivaldi") {
                    "VIVALDI TABS"
                } else {
                    "BROWSER TABS"
                };
                browser_tab_map.entry(browser.to_string()).or_default().push((tab_name.clone(), *duration, *is_focus));
            }
            for (browser, tabs) in browser_tab_map {
                println!("~=~ {} (TOP 5) ~=~\n", browser);
                let max_duration = tabs.first().map(|(_, d, _)| d.as_secs()).unwrap_or(1);
                for (tab_name, duration, is_focus) in tabs.iter().take(5) {
                    let tab_display = if tab_name.len() > 30 { format!("{}...", &tab_name[..27]) } else { tab_name.clone() };
                    let duration_str = Self::format_duration(*duration);
                    let focus_text = if *is_focus { "Focus" } else { "Other" };
                    let bar_len = 15;
                    let filled = ((duration.as_secs() as f64 / max_duration as f64) * bar_len as f64) as usize;
                    let usage_bar = format!("[{}{}]", "■".repeat(filled), "□".repeat(bar_len - filled));
                    let tab_line = format!("{:<30} {} {:<8} ({:<5})", tab_display, usage_bar, duration_str, focus_text);
                    println!("{}\n", tab_line);
                }
            }
        }
        println!("{}\n", top_sep);
        println!("~=~ Use 'focusdebt stats' to see your recent progress\n");
    }

    // Helper function to detect browser applications
    fn is_browser_app(app_name: &str) -> bool {
        let browser_apps = ["chrome", "firefox", "safari", "edge", "brave", "chromium", "opera", "vivaldi"];
        browser_apps.iter().any(|&browser| app_name.to_lowercase().contains(browser))
    }
} 