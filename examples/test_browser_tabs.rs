mod stats;
mod tracking;
use stats::{Stats, AggregatedSession};
use std::time::Duration;
use chrono::Utc;

fn main() {
    // Create a mock session with browser and non-browser apps
    let session = create_mock_session();
    
    // Display the session summary to see the new format
    println!("Testing new browser tab display format:");
    println!("=====================================");
    Stats::display_session_summary(&session);
}

fn create_mock_session() -> AggregatedSession {
    let now = Utc::now();
    let start_time = now - chrono::Duration::hours(2);
    
    AggregatedSession {
        session_name: "Test Work Session".to_string(),
        start_time,
        end_time: Some(now),
        total_duration: Duration::from_secs(7200), // 2 hours
        focus_efficiency: 65.0,
        app_usage: vec![
            // Browser apps (should be filtered out from APPLICATIONS USED)
            ("chrome".to_string(), Duration::from_secs(3600), false), // 1 hour
            ("firefox".to_string(), Duration::from_secs(1800), false), // 30 min
            
            // Regular apps (should appear in APPLICATIONS USED)
            ("code".to_string(), Duration::from_secs(1800), true), // 30 min (focus)
            ("terminal".to_string(), Duration::from_secs(300), false), // 5 min
        ],
        domain_usage: vec![
            // Browser domains (should appear in CHROME TABS)
            ("github.com".to_string(), Duration::from_secs(1800), true), // 30 min (focus)
            ("stackoverflow.com".to_string(), Duration::from_secs(1200), true), // 20 min (focus)
            ("youtube.com".to_string(), Duration::from_secs(600), false), // 10 min (distraction)
            ("reddit.com".to_string(), Duration::from_secs(300), false), // 5 min (distraction)
        ],
        context_switches: 15,
    }
} 