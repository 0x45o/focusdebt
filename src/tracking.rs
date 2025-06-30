use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSession {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub app_name: String,
    pub window_title: String,
    pub duration: Duration,
    pub is_focus_app: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSwitch {
    pub timestamp: DateTime<Utc>,
    pub from_app: String,
    pub to_app: String,
    pub recovery_time: Option<Duration>,
}

pub struct FocusTracker {
    current_session: Option<FocusSession>,
    context_switches: Vec<ContextSwitch>,
    focus_apps: Vec<String>,
    last_switch_time: Option<Instant>,
    is_tracking: bool,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            current_session: None,
            context_switches: Vec::new(),
            focus_apps: Vec::new(),
            last_switch_time: None,
            is_tracking: false,
        }
    }

    pub fn start_tracking(&mut self) {
        self.is_tracking = true;
        println!("Focus tracking started");
    }

    pub fn stop_tracking(&mut self) {
        self.is_tracking = false;
        if let Some(session) = &mut self.current_session {
            session.end_time = Some(Utc::now());
            session.duration = Utc::now().signed_duration_since(session.start_time).to_std().unwrap_or(Duration::ZERO);
        }
        println!("Focus tracking stopped");
    }

    pub fn is_tracking(&self) -> bool {
        self.is_tracking
    }

    pub fn add_focus_app(&mut self, app_name: String) {
        if !self.focus_apps.contains(&app_name) {
            self.focus_apps.push(app_name);
        }
    }

    pub fn remove_focus_app(&mut self, app_name: &str) {
        self.focus_apps.retain(|app| app != app_name);
    }

    pub fn list_focus_apps(&self) -> &[String] {
        &self.focus_apps
    }

    pub fn get_focus_apps(&self) -> &[String] {
        &self.focus_apps
    }

    pub fn update_active_window(&mut self, app_name: String, window_title: String) {
        if !self.is_tracking {
            return;
        }

        let now = Utc::now();
        let is_focus_app = self.focus_apps.contains(&app_name);

        if let Some(current_session) = &mut self.current_session {
            if current_session.app_name != app_name {
                // Calculate recovery time if switching to a focus app
                let recovery_time = if is_focus_app && self.last_switch_time.is_some() {
                    Some(Instant::now().duration_since(self.last_switch_time.unwrap()))
                } else {
                    None
                };

                let switch = ContextSwitch {
                    timestamp: now,
                    from_app: current_session.app_name.clone(),
                    to_app: app_name.clone(),
                    recovery_time,
                };
                self.context_switches.push(switch);

                // End current session
                current_session.end_time = Some(now);
                current_session.duration = now.signed_duration_since(current_session.start_time).to_std().unwrap_or(Duration::ZERO);

                // Start new session
                self.current_session = Some(FocusSession {
                    start_time: now,
                    end_time: None,
                    app_name,
                    window_title,
                    duration: Duration::ZERO,
                    is_focus_app,
                });

                // Update last switch time
                self.last_switch_time = Some(Instant::now());
            } else {
                // Same app, update window title
                current_session.window_title = window_title;
            }
        } else {
            // First session
            self.current_session = Some(FocusSession {
                start_time: now,
                end_time: None,
                app_name,
                window_title,
                duration: Duration::ZERO,
                is_focus_app,
            });
        }
    }

    pub fn get_current_session(&self) -> Option<&FocusSession> {
        self.current_session.as_ref()
    }

    pub fn get_context_switches(&self) -> &[ContextSwitch] {
        &self.context_switches
    }

    pub fn get_deep_focus_sessions(&self, _min_duration: Duration) -> Vec<&FocusSession> {
        // This would need to be implemented with database queries
        // For now, return empty vector
        Vec::new()
    }

    pub fn get_average_recovery_time(&self) -> Option<Duration> {
        let recovery_times: Vec<Duration> = self.context_switches
            .iter()
            .filter_map(|switch| switch.recovery_time)
            .collect();
        
        if recovery_times.is_empty() {
            None
        } else {
            let total: Duration = recovery_times.iter().sum();
            Some(Duration::from_secs(total.as_secs() / recovery_times.len() as u64))
        }
    }
}

// Platform-specific window tracking
#[cfg(target_os = "linux")]
pub mod platform {
    use std::process::Command;

    pub fn get_active_window() -> Option<(String, String)> {
        // Try to get active window using xdotool with proper error handling
        let output = Command::new("xdotool")
            .args(&["getactivewindow", "getwindowname"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let window_title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if window_title.is_empty() {
            return None;
        }
        
        // Try to get the process name
        let pid_output = Command::new("xdotool")
            .args(&["getactivewindow", "getwindowpid"])
            .output()
            .ok()?;

        if !pid_output.status.success() {
            return None;
        }

        let pid = String::from_utf8_lossy(&pid_output.stdout).trim().to_string();
        if pid.is_empty() {
            return None;
        }

        // Validate PID is numeric
        if !pid.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        if let Ok(_pid_num) = pid.parse::<u32>() {
            let app_output = Command::new("ps")
                .args(&["-p", &pid, "-o", "comm="])
                .output()
                .ok()?;

            if app_output.status.success() {
                let app_name = String::from_utf8_lossy(&app_output.stdout).trim().to_string();
                if !app_name.is_empty() {
                    return Some((app_name, window_title));
                }
            }
        }

        None
    }
}

#[cfg(target_os = "macos")]
pub mod platform {
    use std::process::Command;

    pub fn get_active_window() -> Option<(String, String)> {
        // More robust AppleScript that handles errors gracefully
        let script = r#"
        try
            tell application "System Events"
                set frontApp to name of first application process whose frontmost is true
            end tell
            
            if frontApp is not missing value then
                try
                    tell application frontApp
                        set window_name to name of front window
                    end tell
                on error
                    set window_name to "Unknown Window"
                end try
                
                return frontApp & "|" & window_name
            else
                return ""
            end if
        on error
            return ""
        end try
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if out.is_empty() {
            return None;
        }

        // Parse the output safely
        let parts: Vec<&str> = out.split('|').collect();
        if parts.len() >= 2 {
            let app_name = parts[0].trim().to_string();
            let window_title = parts[1].trim().to_string();
            
            if !app_name.is_empty() {
                return Some((app_name, window_title));
            }
        }

        None
    }
}

#[cfg(target_os = "windows")]
pub mod platform {
    use std::process::Command;

    pub fn get_active_window() -> Option<(String, String)> {
        // PowerShell script to get both window title and process name
        let script = r#"
        Add-Type @"
        using System;
        using System.Runtime.InteropServices;
        using System.Text;
        
        public class Win32 {
            [DllImport("user32.dll")]
            public static extern IntPtr GetForegroundWindow();
            
            [DllImport("user32.dll")]
            public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
            
            [DllImport("user32.dll")]
            public static extern int GetWindowTextLength(IntPtr hWnd);
            
            [DllImport("user32.dll")]
            public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
        }
"@

        try {
            $h = [Win32]::GetForegroundWindow()
            $len = [Win32]::GetWindowTextLength($h)
            $sb = New-Object System.Text.StringBuilder -ArgumentList ($len + 1)
            [Win32]::GetWindowText($h, $sb, $sb.Capacity) | Out-Null
            $windowTitle = $sb.ToString()
            
            $processId = 0
            [Win32]::GetWindowThreadProcessId($h, [ref]$processId) | Out-Null
            
            if ($processId -gt 0) {
                $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
                if ($process) {
                    $appName = $process.ProcessName
                    return "$appName|$windowTitle"
                }
            }
            
            return "UnknownApp|$windowTitle"
        }
        catch {
            return ""
        }
        "#;

        let output = Command::new("powershell")
            .args(&["-Command", script])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if out.is_empty() {
            return None;
        }

        // Parse the output safely
        let parts: Vec<&str> = out.split('|').collect();
        if parts.len() >= 2 {
            let app_name = parts[0].trim().to_string();
            let window_title = parts[1].trim().to_string();
            
            if !app_name.is_empty() && app_name != "UnknownApp" {
                return Some((app_name, window_title));
            }
        }

        None
    }
} 