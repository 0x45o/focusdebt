use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc, Datelike, Weekday};
use crate::tracking::{FocusSession, ContextSwitch};
use crate::storage::Database;

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
    pub average_recovery_time: Option<Duration>,
}

#[derive(Debug)]
pub struct WeeklyStats {
    pub week_start: DateTime<Utc>,
    pub daily_stats: Vec<DailyStats>,
    pub total_focus_time: Duration,
    pub total_distraction_time: Duration,
    pub total_context_switches: usize,
    pub average_daily_focus: Duration,
    pub average_daily_switches: f64,
    pub total_deep_focus_sessions: usize,
    pub average_recovery_time: Option<Duration>,
}

pub struct Stats;

impl Stats {
    pub fn calculate_daily_stats(db: &Database, date: DateTime<Utc>) -> Result<DailyStats, Box<dyn std::error::Error>> {
        let sessions = db.get_sessions_for_date(date)?;
        let switches = db.get_context_switches_for_date(date)?;
        let deep_sessions = db.get_deep_focus_sessions(30 * 60, date)?; // 30 minutes
        let most_distracting = db.get_most_distracting_apps(date, 5)?;
        let avg_recovery = db.get_average_recovery_time(date)?;

        let mut total_focus_time = Duration::ZERO;
        let mut total_distraction_time = Duration::ZERO;
        let mut app_usage: HashMap<String, Duration> = HashMap::new();

        for session in &sessions {
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

        // Sort apps by usage time
        let mut app_usage_vec: Vec<(String, Duration)> = app_usage.into_iter().collect();
        app_usage_vec.sort_by(|a, b| b.1.cmp(&a.1));

        let most_used_apps = app_usage_vec.iter()
            .filter(|(_, duration)| *duration > Duration::ZERO)
            .take(5)
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
            most_distracting_apps: most_distracting,
            average_recovery_time: avg_recovery,
        })
    }

    pub fn calculate_weekly_stats(db: &Database, week_start: DateTime<Utc>) -> Result<WeeklyStats, Box<dyn std::error::Error>> {
        let mut daily_stats = Vec::new();
        let mut total_focus_time: Duration = Duration::ZERO;
        let mut total_distraction_time: Duration = Duration::ZERO;
        let mut total_context_switches = 0;
        let mut total_deep_focus_sessions = 0;
        let mut recovery_times = Vec::new();

        // Calculate stats for each day of the week
        for day_offset in 0..7 {
            let date = week_start + chrono::Duration::days(day_offset);
            let daily = Self::calculate_daily_stats(db, date)?;
            
            total_focus_time += daily.total_focus_time;
            total_distraction_time += daily.total_distraction_time;
            total_context_switches += daily.context_switches;
            total_deep_focus_sessions += daily.deep_focus_sessions;
            
            if let Some(recovery) = daily.average_recovery_time {
                recovery_times.push(recovery);
            }
            
            daily_stats.push(daily);
        }

        let average_daily_focus = if !daily_stats.is_empty() {
            Duration::from_secs(total_focus_time.as_secs() / daily_stats.len() as u64)
        } else {
            Duration::ZERO
        };

        let average_daily_switches = if !daily_stats.is_empty() {
            total_context_switches as f64 / daily_stats.len() as f64
        } else {
            0.0
        };

        let average_recovery_time = if !recovery_times.is_empty() {
            let total: Duration = recovery_times.iter().sum();
            Some(Duration::from_secs(total.as_secs() / recovery_times.len() as u64))
        } else {
            None
        };

        Ok(WeeklyStats {
            week_start,
            daily_stats,
            total_focus_time,
            total_distraction_time,
            total_context_switches,
            average_daily_focus,
            average_daily_switches,
            total_deep_focus_sessions,
            average_recovery_time,
        })
    }

    pub fn format_duration(duration: Duration) -> String {
        let hours = duration.as_secs() / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    pub fn display_daily_stats(stats: &DailyStats) {
        println!("📊 Focus Report for {}", stats.date.format("%Y-%m-%d"));
        println!("{}", "=".repeat(50));
        
        println!("⏱️  Focus Time: {}", Self::format_duration(stats.total_focus_time));
        println!("🎯 Distraction Time: {}", Self::format_duration(stats.total_distraction_time));
        println!("🔄 Context Switches: {}", stats.context_switches);
        println!("🧠 Deep Focus Sessions: {}", stats.deep_focus_sessions);
        println!("📈 Focus Efficiency: {:.1}%", stats.focus_efficiency);
        
        if let Some(recovery) = stats.average_recovery_time {
            println!("⏰ Average Recovery Time: {}", Self::format_duration(recovery));
        }
        
        if !stats.most_used_apps.is_empty() {
            println!("\n🏆 Most Used Apps:");
            for (i, (app, duration)) in stats.most_used_apps.iter().enumerate() {
                println!("  {}. {} - {}", i + 1, app, Self::format_duration(*duration));
            }
        }

        if !stats.most_distracting_apps.is_empty() {
            println!("\n⚠️  Most Distracting Apps:");
            for (i, (app, duration)) in stats.most_distracting_apps.iter().enumerate() {
                println!("  {}. {} - {}", i + 1, app, Self::format_duration(*duration));
            }
        }
    }

    pub fn display_weekly_stats(stats: &WeeklyStats) {
        println!("📊 Weekly Focus Report ({} - {})", 
            stats.week_start.format("%Y-%m-%d"),
            stats.week_start + chrono::Duration::days(6));
        println!("{}", "=".repeat(60));
        
        println!("⏱️  Total Focus Time: {}", Self::format_duration(stats.total_focus_time));
        println!("🎯 Total Distraction Time: {}", Self::format_duration(stats.total_distraction_time));
        println!("🔄 Total Context Switches: {}", stats.total_context_switches);
        println!("🧠 Total Deep Focus Sessions: {}", stats.total_deep_focus_sessions);
        println!("📈 Average Daily Focus: {}", Self::format_duration(stats.average_daily_focus));
        println!("🔄 Average Daily Switches: {:.1}", stats.average_daily_switches);
        
        if let Some(recovery) = stats.average_recovery_time {
            println!("⏰ Average Recovery Time: {}", Self::format_duration(recovery));
        }
        
        println!("\n📅 Daily Breakdown:");
        for daily in &stats.daily_stats {
            let day_name = daily.date.format("%a").to_string();
            let focus_percent = if daily.total_focus_time + daily.total_distraction_time > Duration::ZERO {
                (daily.total_focus_time.as_secs_f64() / (daily.total_focus_time + daily.total_distraction_time).as_secs_f64() * 100.0) as i32
            } else {
                0
            };
            println!("  {}: {} focus ({}%) | {} switches | {} deep sessions", 
                day_name, 
                Self::format_duration(daily.total_focus_time),
                focus_percent,
                daily.context_switches,
                daily.deep_focus_sessions);
        }
    }

    pub fn generate_ascii_report(stats: &DailyStats) -> String {
        let mut report = String::new();
        
        // Header
        report.push_str("╔══════════════════════════════════════════════════════════════╗\n");
        report.push_str(&format!("║                    FOCUS REPORT - {}                    ║\n", stats.date.format("%Y-%m-%d")));
        report.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        
        // Focus metrics
        report.push_str(&format!("║  🧠 Focus Time: {:<45} ║\n", Self::format_duration(stats.total_focus_time)));
        report.push_str(&format!("║  🎯 Distraction Time: {:<40} ║\n", Self::format_duration(stats.total_distraction_time)));
        report.push_str(&format!("║  🔄 Context Switches: {:<42} ║\n", stats.context_switches));
        report.push_str(&format!("║  🧠 Deep Focus Sessions: {:<38} ║\n", stats.deep_focus_sessions));
        report.push_str(&format!("║  📈 Focus Efficiency: {:.1}%{:<40} ║\n", stats.focus_efficiency, ""));
        
        if let Some(recovery) = stats.average_recovery_time {
            report.push_str(&format!("║  ⏰ Avg Recovery Time: {:<40} ║\n", Self::format_duration(recovery)));
        }
        
        // Progress bar for focus efficiency
        let bar_length = 40;
        let filled_length = ((stats.focus_efficiency / 100.0) * bar_length as f64) as usize;
        let bar = "█".repeat(filled_length) + &"░".repeat(bar_length - filled_length);
        report.push_str(&format!("║  [{}] {:<35} ║\n", bar, ""));
        
        report.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        
        // Most used apps
        if !stats.most_used_apps.is_empty() {
            report.push_str("║  🏆 Most Used Apps:                                        ║\n");
            for (i, (app, duration)) in stats.most_used_apps.iter().take(3).enumerate() {
                let app_display = if app.len() > 20 { &app[..20] } else { app };
                report.push_str(&format!("║     {}. {:<20} {:<25} ║\n", 
                    i + 1, app_display, Self::format_duration(*duration)));
            }
        }
        
        report.push_str("╚══════════════════════════════════════════════════════════════╝\n");
        
        report
    }
} 