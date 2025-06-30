use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::storage::Database;
use crate::tracking::{FocusSession, ContextSwitch};
use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub export_date: DateTime<Utc>,
    pub date_range: DateRange,
    pub sessions: Vec<FocusSession>,
    pub context_switches: Vec<ContextSwitch>,
    pub summary: ExportSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportSummary {
    pub total_focus_time_seconds: u64,
    pub total_distraction_time_seconds: u64,
    pub total_context_switches: usize,
    pub deep_focus_sessions: usize,
    pub focus_efficiency_percentage: f64,
    pub average_recovery_time_seconds: Option<u64>,
}

pub struct Exporter;

impl Exporter {
    pub fn export_data(
        db: &Database,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        format: &str,
        output_path: Option<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Load data from database
        let sessions = Self::get_sessions_in_range(db, start_date, end_date)?;
        let switches = Self::get_switches_in_range(db, start_date, end_date)?;
        
        // Calculate summary
        let summary = Self::calculate_summary(&sessions, &switches);
        
        let export_data = ExportData {
            export_date: Utc::now(),
            date_range: DateRange {
                start: start_date,
                end: end_date,
            },
            sessions,
            context_switches: switches,
            summary,
        };

        // Determine output path
        let output_path = output_path.unwrap_or_else(|| {
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("focusdebt_export_{}.{}", timestamp, format);
            Config::load()
                .map(|config| config.get_export_path().join(&filename))
                .unwrap_or_else(|_| PathBuf::from(filename))
        });

        // Ensure export directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Export based on format
        match format.to_lowercase().as_str() {
            "json" => Self::export_json(&export_data, &output_path)?,
            "csv" => Self::export_csv(&export_data, &output_path)?,
            "html" => Self::export_html(&export_data, &output_path)?,
            _ => return Err("Unsupported export format. Use: json, csv, or html".into()),
        }

        println!("✅ Data exported to: {}", output_path.display());
        Ok(())
    }

    fn get_sessions_in_range(
        db: &Database,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<FocusSession>, Box<dyn std::error::Error>> {
        let mut all_sessions = Vec::new();
        let mut current_date = start_date;

        while current_date <= end_date {
            let sessions = db.get_sessions_for_date(current_date)?;
            all_sessions.extend(sessions);
            current_date += chrono::Duration::days(1);
        }

        Ok(all_sessions)
    }

    fn get_switches_in_range(
        db: &Database,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<ContextSwitch>, Box<dyn std::error::Error>> {
        let mut all_switches = Vec::new();
        let mut current_date = start_date;

        while current_date <= end_date {
            let switches = db.get_context_switches_for_date(current_date)?;
            all_switches.extend(switches);
            current_date += chrono::Duration::days(1);
        }

        Ok(all_switches)
    }

    fn calculate_summary(sessions: &[FocusSession], switches: &[ContextSwitch]) -> ExportSummary {
        let mut total_focus_time = 0u64;
        let mut total_distraction_time = 0u64;
        let mut deep_focus_sessions = 0;

        for session in sessions {
            let duration_seconds = session.duration.as_secs();
            if session.is_focus_app {
                total_focus_time += duration_seconds;
                if duration_seconds >= 30 * 60 { // 30 minutes
                    deep_focus_sessions += 1;
                }
            } else {
                total_distraction_time += duration_seconds;
            }
        }

        let total_time = total_focus_time + total_distraction_time;
        let focus_efficiency = if total_time > 0 {
            (total_focus_time as f64 / total_time as f64) * 100.0
        } else {
            0.0
        };

        let recovery_times: Vec<u64> = switches
            .iter()
            .filter_map(|switch| switch.recovery_time.map(|d| d.as_secs()))
            .collect();

        let average_recovery_time = if !recovery_times.is_empty() {
            let total: u64 = recovery_times.iter().sum();
            Some(total / recovery_times.len() as u64)
        } else {
            None
        };

        ExportSummary {
            total_focus_time_seconds: total_focus_time,
            total_distraction_time_seconds: total_distraction_time,
            total_context_switches: switches.len(),
            deep_focus_sessions,
            focus_efficiency_percentage: focus_efficiency,
            average_recovery_time_seconds: average_recovery_time,
        }
    }

    fn export_json(data: &ExportData, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(data)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn export_csv(data: &ExportData, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut csv_content = String::new();
        
        // Add summary
        csv_content.push_str("Summary\n");
        csv_content.push_str(&format!("Total Focus Time (seconds),{}\n", data.summary.total_focus_time_seconds));
        csv_content.push_str(&format!("Total Distraction Time (seconds),{}\n", data.summary.total_distraction_time_seconds));
        csv_content.push_str(&format!("Total Context Switches,{}\n", data.summary.total_context_switches));
        csv_content.push_str(&format!("Deep Focus Sessions,{}\n", data.summary.deep_focus_sessions));
        csv_content.push_str(&format!("Focus Efficiency (%),{:.2}\n", data.summary.focus_efficiency_percentage));
        if let Some(recovery) = data.summary.average_recovery_time_seconds {
            csv_content.push_str(&format!("Average Recovery Time (seconds),{}\n", recovery));
        }
        csv_content.push_str("\n");

        // Add sessions
        csv_content.push_str("Sessions\n");
        csv_content.push_str("Start Time,End Time,App Name,Window Title,Duration (seconds),Is Focus App\n");
        for session in &data.sessions {
            let end_time = session.end_time
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "".to_string());
            
            csv_content.push_str(&format!(
                "{},{},{},{},{},{}\n",
                session.start_time.format("%Y-%m-%d %H:%M:%S"),
                end_time,
                session.app_name,
                session.window_title.replace(",", ";"),
                session.duration.as_secs(),
                session.is_focus_app
            ));
        }
        csv_content.push_str("\n");

        // Add context switches
        csv_content.push_str("Context Switches\n");
        csv_content.push_str("Timestamp,From App,To App,Recovery Time (seconds)\n");
        for switch in &data.context_switches {
            let recovery_time = switch.recovery_time
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|| "".to_string());
            
            csv_content.push_str(&format!(
                "{},{},{},{}\n",
                switch.timestamp.format("%Y-%m-%d %H:%M:%S"),
                switch.from_app,
                switch.to_app,
                recovery_time
            ));
        }

        fs::write(path, csv_content)?;
        Ok(())
    }

    fn export_html(data: &ExportData, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>FocusDebt Export - {} to {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .summary {{ background: #f5f5f5; padding: 20px; border-radius: 8px; margin-bottom: 30px; }}
        .metric {{ display: inline-block; margin: 10px 20px 10px 0; }}
        .metric-value {{ font-size: 24px; font-weight: bold; color: #2c3e50; }}
        .metric-label {{ font-size: 12px; color: #7f8c8d; text-transform: uppercase; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #34495e; color: white; }}
        tr:nth-child(even) {{ background-color: #f9f9f9; }}
        .focus {{ color: #27ae60; }}
        .distraction {{ color: #e74c3c; }}
    </style>
</head>
<body>
    <h1>FocusDebt Export Report</h1>
    <p>Generated on: {}</p>
    <p>Date Range: {} to {}</p>
    
    <div class="summary">
        <h2>Summary</h2>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Focus Time</div>
        </div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Distraction Time</div>
        </div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Context Switches</div>
        </div>
        <div class="metric">
            <div class="metric-value">{}</div>
            <div class="metric-label">Deep Focus Sessions</div>
        </div>
        <div class="metric">
            <div class="metric-value">{:.1}%</div>
            <div class="metric-label">Focus Efficiency</div>
        </div>
    </div>

    <h2>Focus Sessions</h2>
    <table>
        <tr>
            <th>Start Time</th>
            <th>End Time</th>
            <th>App</th>
            <th>Window</th>
            <th>Duration</th>
            <th>Type</th>
        </tr>
        {}
    </table>

    <h2>Context Switches</h2>
    <table>
        <tr>
            <th>Time</th>
            <th>From</th>
            <th>To</th>
            <th>Recovery Time</th>
        </tr>
        {}
    </table>
</body>
</html>"#,
            data.date_range.start.format("%Y-%m-%d"),
            data.date_range.end.format("%Y-%m-%d"),
            data.export_date.format("%Y-%m-%d %H:%M:%S"),
            data.date_range.start.format("%Y-%m-%d %H:%M:%S"),
            data.date_range.end.format("%Y-%m-%d %H:%M:%S"),
            Self::format_duration(data.summary.total_focus_time_seconds),
            Self::format_duration(data.summary.total_distraction_time_seconds),
            data.summary.total_context_switches,
            data.summary.deep_focus_sessions,
            data.summary.focus_efficiency_percentage,
            Self::generate_sessions_html(&data.sessions),
            Self::generate_switches_html(&data.context_switches),
        );

        fs::write(path, html)?;
        Ok(())
    }

    fn format_duration(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    fn generate_sessions_html(sessions: &[FocusSession]) -> String {
        sessions.iter().map(|session| {
            let end_time = session.end_time
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "Active".to_string());
            
            let class = if session.is_focus_app { "focus" } else { "distraction" };
            let type_text = if session.is_focus_app { "Focus" } else { "Distraction" };
            
            format!(
                "<tr class=\"{}\">
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>",
                class,
                session.start_time.format("%H:%M:%S"),
                end_time,
                session.app_name,
                session.window_title,
                Self::format_duration(session.duration.as_secs()),
                type_text
            )
        }).collect::<Vec<_>>().join("\n")
    }

    fn generate_switches_html(switches: &[ContextSwitch]) -> String {
        switches.iter().map(|switch| {
            let recovery_time = switch.recovery_time
                .map(|d| Self::format_duration(d.as_secs()))
                .unwrap_or_else(|| "N/A".to_string());
            
            format!(
                "<tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>",
                switch.timestamp.format("%H:%M:%S"),
                switch.from_app,
                switch.to_app,
                recovery_time
            )
        }).collect::<Vec<_>>().join("\n")
    }
} 