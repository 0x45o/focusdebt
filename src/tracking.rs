use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FocusSession {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub app_name: String,
    pub window_title: String,
    pub domain: Option<String>,
    pub duration: Duration,
    pub is_focus_app: bool,
    pub session_name: String,
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
    completed_sessions: Vec<FocusSession>,
    context_switches: Vec<ContextSwitch>,
    focus_apps: Vec<String>,
    focus_sites: Vec<String>,
    last_switch_time: Option<Instant>,
    is_tracking: bool,
    debug_mode: bool,
    session_name: String,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            current_session: None,
            completed_sessions: Vec::new(),
            context_switches: Vec::new(),
            focus_apps: Vec::new(),
            focus_sites: Vec::new(),
            last_switch_time: None,
            is_tracking: false,
            debug_mode: true, // Enable debug mode by default
            session_name: String::new(),
        }
    }

    pub fn is_browser_app(app_name: &str) -> bool {
        let browser_apps = ["chrome", "firefox", "safari", "edge", "brave", "chromium", "opera", "vivaldi"];
        browser_apps.iter().any(|&browser| app_name.to_lowercase().contains(browser))
    }

    pub fn start_tracking(&mut self) {
        self.is_tracking = true;
        println!("~=~ Focus tracking started (debug mode: {})", self.debug_mode);
    }

    pub fn stop_tracking(&mut self) {
        self.is_tracking = false;
        if let Some(session) = &mut self.current_session {
            let now = Utc::now();
            session.end_time = Some(now);
            session.duration = now.signed_duration_since(session.start_time).to_std().unwrap_or(Duration::ZERO);
            if self.debug_mode {
                println!("~=~ Ending session: {} ({}s)", session.app_name, session.duration.as_secs());
            }
        }
        println!("~=~ Focus tracking stopped");
    }

    pub fn is_tracking(&self) -> bool {
        self.is_tracking
    }

    pub fn add_focus_app(&mut self, app_name: String) {
        if !self.focus_apps.contains(&app_name) {
            self.focus_apps.push(app_name.clone());
            if self.debug_mode {
                println!("~=~ Added focus app: {}", app_name);
            }
        }
    }

    pub fn remove_focus_app(&mut self, app_name: &str) {
        self.focus_apps.retain(|app| app != app_name);
        if self.debug_mode {
            println!("~=~ Removed focus app: {}", app_name);
        }
    }

    pub fn list_focus_apps(&self) -> &[String] {
        &self.focus_apps
    }

    pub fn get_focus_apps(&self) -> &[String] {
        &self.focus_apps
    }

    pub fn add_focus_site(&mut self, domain: String) {
        if !self.focus_sites.contains(&domain) {
            self.focus_sites.push(domain.clone());
            if self.debug_mode {
                println!("~=~ Added focus site: {}", domain);
            }
        }
    }

    pub fn remove_focus_site(&mut self, domain: &str) {
        self.focus_sites.retain(|s| s != domain);
        if self.debug_mode {
            println!("~=~ Removed focus site: {}", domain);
        }
    }

    pub fn list_focus_sites(&self) -> &[String] {
        &self.focus_sites
    }

    pub fn get_focus_sites(&self) -> &[String] {
        &self.focus_sites
    }

    pub fn update_active_window(&mut self, app_name: String, window_title: String) {
        if !self.is_tracking {
            return;
        }

        let now = Utc::now();
        
        // For browsers, store the window title (tab name); for non-browsers, no domain tracking
        let domain = if Self::is_browser_app(&app_name) {
            Some(window_title.clone())  // Store the full tab name for browsers
        } else {
            None  // No domain tracking for non-browsers
        };
        
        // Determine if this is a focus session based on app and/or tab name
        let mut is_focus_app = self.focus_apps.contains(&app_name);
        
        // If we have a tab name and it's in focus sites, mark as focus (case-insensitive)
        if let Some(ref tab_name) = domain {
            if self.focus_sites.iter().any(|site| tab_name.to_lowercase().contains(&site.to_lowercase())) {
                is_focus_app = true;
            }
        }

        if self.debug_mode {
            let is_browser = Self::is_browser_app(&app_name);
            let debug_msg = format!("~=~ BROWSER CHECK: {} - is_browser: {}, tab_name: {:?}", app_name, is_browser, domain);
            println!("{}", debug_msg);
            // Also write to debug file for visibility
            let _ = std::fs::write("/tmp/focusdebt_debug.log", format!("{}\n", debug_msg));
        }

        if self.debug_mode {
            let debug_msg = if let Some(ref tab_name) = domain {
                format!("~=~ Window update: {} - {} (tab_name: {}, focus: {})", 
                    app_name, window_title, tab_name, is_focus_app)
            } else {
                format!("~=~ Window update: {} - {} (focus: {})", app_name, window_title, is_focus_app)
            };
            println!("{}", debug_msg);
            // Also write to debug file for visibility
            let _ = std::fs::write("/tmp/focusdebt_debug.log", format!("{}\n", debug_msg));
        }

        if let Some(current_session) = &mut self.current_session {
            // Check if we're switching to a different app OR different browser tab/domain
            let is_browser = Self::is_browser_app(&app_name);
            let is_browser_tab_change = is_browser && (
                current_session.window_title != window_title ||
                current_session.domain != domain
            );
            
            if current_session.app_name != app_name || is_browser_tab_change {
                if self.debug_mode {
                    if current_session.app_name != app_name {
                        println!("~=~ App switch detected: {} → {}", current_session.app_name, app_name);
                    } else {
                        println!("~=~ Browser tab switch detected: {} → {}", 
                            current_session.window_title, window_title);
                    }
                }

                // Calculate recovery time if switching to a focus app
                let recovery_time = if is_focus_app && self.last_switch_time.is_some() {
                    Some(Instant::now().duration_since(self.last_switch_time.unwrap()))
                } else {
                    None
                };

                // Create context switch record
                let switch = ContextSwitch {
                    timestamp: now,
                    from_app: current_session.app_name.clone(),
                    to_app: app_name.clone(),
                    recovery_time,
                };
                self.context_switches.push(switch);

                if self.debug_mode {
                    if let Some(recovery) = recovery_time {
                        println!("~=~ Recovery time: {}s", recovery.as_secs());
                    }
                }

                // End current session and add to completed sessions
                current_session.end_time = Some(now);
                current_session.duration = now.signed_duration_since(current_session.start_time).to_std().unwrap_or(Duration::ZERO);
                
                let completed_session = current_session.clone();
                self.completed_sessions.push(completed_session);

                if self.debug_mode {
                    println!("~=~ Completed session: {} ({}s)", 
                        current_session.app_name, 
                        current_session.duration.as_secs()
                    );
                }

                // Start new session
                self.current_session = Some(FocusSession {
                    start_time: now,
                    end_time: None,
                    app_name: app_name.clone(),
                    window_title,
                    domain: domain.clone(),
                    duration: Duration::ZERO,
                    is_focus_app,
                    session_name: self.session_name.clone(),
                });

                // Update last switch time
                self.last_switch_time = Some(Instant::now());

                if self.debug_mode {
                    println!("~=~ Started new session: {}", app_name);
                }
            } else {
                // Same app and same browser tab, just update window title if it changed
                if current_session.window_title != window_title {
                    if self.debug_mode {
                        println!("~=~ Window title update: {} → {}", current_session.window_title, window_title);
                    }
                    current_session.window_title = window_title;
                }
            }
        } else {
            // First session
            self.current_session = Some(FocusSession {
                start_time: now,
                end_time: None,
                app_name: app_name.clone(),
                window_title,
                domain: domain.clone(),
                duration: Duration::ZERO,
                is_focus_app,
                session_name: self.session_name.clone(),
            });

            if self.debug_mode {
                println!("~=~ Started first session: {}", app_name);
            }
        }
    }

    pub fn end_current_session(&mut self) {
        if let Some(session) = &mut self.current_session {
            if session.end_time.is_none() {
                let now = Utc::now();
                session.end_time = Some(now);
                session.duration = now.signed_duration_since(session.start_time).to_std().unwrap_or(Duration::ZERO);
                
                let completed_session = session.clone();
                self.completed_sessions.push(completed_session);

                if self.debug_mode {
                    println!("~=~ Manually ended session: {} ({}s)", 
                        session.app_name, 
                        session.duration.as_secs()
                    );
                }
            }
        }
    }

    pub fn get_current_session(&self) -> Option<FocusSession> {
        self.current_session.as_ref().map(|session| {
            let mut updated_session = session.clone();
            // Only update duration if session hasn't ended yet
            if updated_session.end_time.is_none() {
                let now = Utc::now();
                updated_session.duration = now.signed_duration_since(session.start_time).to_std().unwrap_or(Duration::ZERO);
            }
            updated_session
        })
    }

    pub fn get_completed_sessions(&self) -> &[FocusSession] {
        &self.completed_sessions
    }

    pub fn take_completed_sessions(&mut self) -> Vec<FocusSession> {
        let sessions = self.completed_sessions.clone();
        self.completed_sessions.clear();
        sessions
    }

    pub fn get_context_switches(&self) -> &[ContextSwitch] {
        &self.context_switches
    }

    pub fn take_context_switches(&mut self) -> Vec<ContextSwitch> {
        let switches = self.context_switches.clone();
        self.context_switches.clear();
        switches
    }

    pub fn get_deep_focus_sessions(&self, _min_duration: Duration) -> Vec<&FocusSession> {
        // This would need to be implemented with database queries
        // For now, return empty vector
        Vec::new()
    }

    pub fn get_stats(&self) -> TrackerStats {
        TrackerStats {
            total_sessions: self.completed_sessions.len(),
            total_context_switches: self.context_switches.len(),
            current_session_duration: self.current_session.as_ref()
                .map(|s| {
                    let now = Utc::now();
                    now.signed_duration_since(s.start_time).to_std().unwrap_or(Duration::ZERO)
                })
                .unwrap_or(Duration::ZERO),
            focus_apps_count: self.focus_apps.len(),
        }
    }

    pub fn set_session_name(&mut self, name: String) {
        self.session_name = name;
        if self.debug_mode {
            println!("~=~ Session name set to: {}", self.session_name);
        }
    }

    pub fn get_session_name(&self) -> &str {
        &self.session_name
    }
}

#[derive(Debug)]
pub struct TrackerStats {
    pub total_sessions: usize,
    pub total_context_switches: usize,
    pub current_session_duration: Duration,
    pub focus_apps_count: usize,
}

// Platform-specific window tracking
#[cfg(target_os = "linux")]
pub mod platform {
    use std::process::Command;
    use std::env;

    pub fn get_active_window() -> Option<(String, String)> {
        let debug = true;
        
        if debug {
            println!("~=~ Detecting Linux window manager and attempting window detection...");
        }

        // Detect the current window manager/compositor environment
        let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let current_desktop = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or_default();
        
        if debug {
            println!("   Session type: {}", session_type);
            println!("   Current desktop: {}", current_desktop);
            println!("   Wayland display: {}", wayland_display);
        }

        // Method 1: Hyprland (Wayland compositor)
        if current_desktop.to_lowercase().contains("hyprland") || 
           env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            if debug {
                println!("~=~ Detected Hyprland, trying hyprctl...");
            }
            
            if let Some(result) = try_hyprland_detection(debug) {
                return Some(result);
            }
        }

        // Method 2: Sway (Wayland compositor)
        if current_desktop.to_lowercase().contains("sway") || 
           env::var("SWAYSOCK").is_ok() {
            if debug {
                println!("~=~ Detected Sway, trying swaymsg...");
            }
            
            if let Some(result) = try_sway_detection(debug) {
                return Some(result);
            }
        }

        // Method 3: GNOME on Wayland
        if session_type == "wayland" && current_desktop.to_lowercase().contains("gnome") {
            if debug {
                println!("~=~ Detected GNOME on Wayland, trying gdbus...");
            }
            
            if let Some(result) = try_gnome_wayland_detection(debug) {
                return Some(result);
            }
        }

        // Method 4: KDE on Wayland
        if session_type == "wayland" && current_desktop.to_lowercase().contains("kde") {
            if debug {
                println!("~=~ Detected KDE on Wayland, trying kwin...");
            }
            
            if let Some(result) = try_kde_wayland_detection(debug) {
                return Some(result);
            }
        }

        // Method 5: Generic Wayland fallback
        if session_type == "wayland" || !wayland_display.is_empty() {
            if debug {
                println!("~=~ Generic Wayland detected, trying wlrctl/wlr-randr...");
            }
            
            if let Some(result) = try_generic_wayland_detection(debug) {
                return Some(result);
            }
        }

        // Method 6: X11 with xdotool (traditional method)
        if debug {
            println!("~=~ Trying X11 detection with xdotool...");
        }
        
        if let Some(result) = try_x11_xdotool_detection(debug) {
            return Some(result);
        }

        // Method 7: X11 with wmctrl fallback
        if debug {
            println!("~=~Trying X11 detection with wmctrl...");
        }
        
        if let Some(result) = try_x11_wmctrl_detection(debug) {
            return Some(result);
        }

        // Method 8: X11 with xprop fallback
        if debug {
            println!("~=~ Trying X11 detection with xprop...");
        }
        
        if let Some(result) = try_x11_xprop_detection(debug) {
            return Some(result);
        }

        // Method 9: Fallback to process scanning
        if debug {
            println!("❌ All methods failed, trying process scanning fallback...");
        }
        
        if let Some(result) = try_process_scanning_fallback(debug) {
            return Some(result);
        }

        if debug {
            eprintln!("❌ All window detection methods failed");
        }
        None
    }

    fn try_hyprland_detection(debug: bool) -> Option<(String, String)> {
        if let Ok(output) = Command::new("hyprctl")
            .args(&["activewindow", "-j"])
            .output() {
            
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                
                // Parse JSON manually (simple approach)
                if let Some(class_start) = json_str.find("\"class\":\"") {
                    if let Some(class_end) = json_str[class_start + 9..].find("\"") {
                        let class_name = &json_str[class_start + 9..class_start + 9 + class_end];
                        
                        if let Some(title_start) = json_str.find("\"title\":\"") {
                            if let Some(title_end) = json_str[title_start + 9..].find("\"") {
                                let title = &json_str[title_start + 9..title_start + 9 + title_end];
                                
                                if debug {
                                    println!("~=~ Hyprland detected: {} - {}", class_name, title);
                                }
                                return Some((class_name.to_string(), title.to_string()));
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback to non-JSON hyprctl
        if let Ok(output) = Command::new("hyprctl")
            .args(&["activewindow"])
            .output() {
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let mut class_name = String::new();
                let mut title = String::new();
                
                for line in output_str.lines() {
                    if line.trim().starts_with("class:") {
                        class_name = line.trim().strip_prefix("class:").unwrap_or("").trim().to_string();
                    } else if line.trim().starts_with("title:") {
                        title = line.trim().strip_prefix("title:").unwrap_or("").trim().to_string();
                    }
                }
                
                if !class_name.is_empty() && !title.is_empty() {
                    if debug {
                        println!("~=~Hyprland detected: {} - {}", class_name, title);
                    }
                    return Some((class_name, title));
                }
            }
        }
        
        None
    }

    fn try_sway_detection(debug: bool) -> Option<(String, String)> {
        if let Ok(output) = Command::new("swaymsg")
            .args(&["-t", "get_tree"])
            .output() {
            
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                
                // Look for focused window in the JSON
                if json_str.contains("\"focused\":true") {
                    // Simple JSON parsing for app_id and name
                    if let Some(app_id_start) = json_str.find("\"app_id\":\"") {
                        if let Some(app_id_end) = json_str[app_id_start + 10..].find("\"") {
                            let app_id = &json_str[app_id_start + 10..app_id_start + 10 + app_id_end];
                            
                            if let Some(name_start) = json_str.find("\"name\":\"") {
                                if let Some(name_end) = json_str[name_start + 8..].find("\"") {
                                    let name = &json_str[name_start + 8..name_start + 8 + name_end];
                                    
                                    if debug {
                                        println!("~=~ Sway detected: {} - {}", app_id, name);
                                    }
                                    return Some((app_id.to_string(), name.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn try_gnome_wayland_detection(debug: bool) -> Option<(String, String)> {
        // Try to get focused window via GNOME Shell's D-Bus interface
        if let Ok(output) = Command::new("gdbus")
            .args(&["call", "--session", "--dest", "org.gnome.Shell", 
                   "--object-path", "/org/gnome/Shell", 
                   "--method", "org.gnome.Shell.Eval", 
                   "global.display.get_focus_window().get_wm_class()"])
            .output() {
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if let Some(class_start) = output_str.find("'") {
                    if let Some(class_end) = output_str[class_start + 1..].find("'") {
                        let class_name = &output_str[class_start + 1..class_start + 1 + class_end];
                        
                        // Get window title
                        if let Ok(title_output) = Command::new("gdbus")
                            .args(&["call", "--session", "--dest", "org.gnome.Shell", 
                                   "--object-path", "/org/gnome/Shell", 
                                   "--method", "org.gnome.Shell.Eval", 
                                   "global.display.get_focus_window().get_title()"])
                            .output() {
                            
                            if title_output.status.success() {
                                let title_str = String::from_utf8_lossy(&title_output.stdout);
                                if let Some(title_start) = title_str.find("'") {
                                    if let Some(title_end) = title_str[title_start + 1..].find("'") {
                                        let title = &title_str[title_start + 1..title_start + 1 + title_end];
                                        
                                        if debug {
                                            println!("~=~ GNOME Wayland detected: {} - {}", class_name, title);
                                        }
                                        return Some((class_name.to_string(), title.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn try_kde_wayland_detection(debug: bool) -> Option<(String, String)> {
        // Try KDE's kwin D-Bus interface
        if let Ok(output) = Command::new("qdbus")
            .args(&["org.kde.KWin", "/KWin", "org.kde.KWin.activeWindow"])
            .output() {
            
            if output.status.success() {
                let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                
                if !window_id.is_empty() {
                    // Get window class
                    if let Ok(class_output) = Command::new("qdbus")
                        .args(&["org.kde.KWin", &format!("/KWin/Window_{}", window_id), 
                               "org.kde.KWin.Window.resourceClass"])
                        .output() {
                        
                        if class_output.status.success() {
                            let class_name = String::from_utf8_lossy(&class_output.stdout).trim().to_string();
                            
                            // Get window title
                            if let Ok(title_output) = Command::new("qdbus")
                                .args(&["org.kde.KWin", &format!("/KWin/Window_{}", window_id), 
                                       "org.kde.KWin.Window.caption"])
                                .output() {
                                
                                if title_output.status.success() {
                                    let title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();
                                    
                                    if debug {
                                        println!("✅ KDE Wayland detected: {} - {}", class_name, title);
                                    }
                                    return Some((class_name, title));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn try_generic_wayland_detection(debug: bool) -> Option<(String, String)> {
        // Try wlr-randr for wlroots-based compositors
        if let Ok(output) = Command::new("wlrctl")
            .args(&["window", "get"])
            .output() {
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Parse wlrctl output for app_id and title
                if let Some(app_id_line) = output_str.lines().find(|line| line.contains("app_id")) {
                    if let Some(title_line) = output_str.lines().find(|line| line.contains("title")) {
                        let app_id = app_id_line.split(':').nth(1).unwrap_or("").trim().to_string();
                        let title = title_line.split(':').nth(1).unwrap_or("").trim().to_string();
                        
                        if !app_id.is_empty() && !title.is_empty() {
                            if debug {
                                println!("~=~ wlrctl detected: {} - {}", app_id, title);
                            }
                            return Some((app_id, title));
                        }
                    }
                }
            }
        }
        
        None
    }

    fn try_x11_xdotool_detection(debug: bool) -> Option<(String, String)> {
        if let Ok(window_id_output) = Command::new("xdotool")
            .args(&["getactivewindow"])
            .output() {
            
            if window_id_output.status.success() {
                let window_id = String::from_utf8_lossy(&window_id_output.stdout).trim().to_string();
                if !window_id.is_empty() && window_id.chars().all(|c| c.is_ascii_digit()) {
                    
                    // Get window title and PID
                    if let (Ok(title_output), Ok(pid_output)) = (
                        Command::new("xdotool").args(&["getwindowname", &window_id]).output(),
                        Command::new("xdotool").args(&["getwindowpid", &window_id]).output()
                    ) {
                        
                        if title_output.status.success() && pid_output.status.success() {
                            let window_title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();
                            let pid = String::from_utf8_lossy(&pid_output.stdout).trim().to_string();
                            
                            if !window_title.is_empty() && !pid.is_empty() {
                                // Get process name from PID
                                if let Ok(ps_output) = Command::new("ps")
                                    .args(&["-p", &pid, "-o", "comm=", "--no-headers"])
                                    .output() {
                                    
                                    if ps_output.status.success() {
                                        let app_name = String::from_utf8_lossy(&ps_output.stdout).trim().to_string();
                                        if !app_name.is_empty() {
                                            if debug {
                                                println!("~=~ xdotool detected: {} - {}", app_name, window_title);
                                            }
                                            return Some((app_name, window_title));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn try_x11_wmctrl_detection(debug: bool) -> Option<(String, String)> {
        if let Ok(output) = Command::new("wmctrl")
            .args(&["-a", "-l"])
            .output() {
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Find the active window (marked with *)
                for line in output_str.lines() {
                    if line.contains("*") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let window_title = parts[3..].join(" ");
                            
                            // Get window class using xprop
                            if let Some(window_id) = parts.get(0) {
                                if let Ok(xprop_output) = Command::new("xprop")
                                    .args(&["-id", window_id, "WM_CLASS"])
                                    .output() {
                                    
                                    if xprop_output.status.success() {
                                        let xprop_str = String::from_utf8_lossy(&xprop_output.stdout);
                                        if let Some(class_start) = xprop_str.find("\"") {
                                            if let Some(class_end) = xprop_str[class_start + 1..].find("\"") {
                                                let app_name = &xprop_str[class_start + 1..class_start + 1 + class_end];
                                                
                                                if debug {
                                                    println!("~=~ wmctrl detected: {} - {}", app_name, window_title);
                                                }
                                                return Some((app_name.to_string(), window_title));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    fn try_x11_xprop_detection(debug: bool) -> Option<(String, String)> {
        if let Ok(xprop_output) = Command::new("bash")
            .args(&["-c", "xprop -id $(xdotool getactivewindow 2>/dev/null) WM_CLASS _NET_WM_NAME 2>/dev/null"])
            .output() {
            
            if xprop_output.status.success() {
                let output_str = String::from_utf8_lossy(&xprop_output.stdout);
                let mut app_name = String::new();
                let mut window_title = String::new();
                
                for line in output_str.lines() {
                    if line.contains("WM_CLASS") {
                        let parts: Vec<&str> = line.split('"').collect();
                        if parts.len() >= 3 {
                            app_name = parts[1].to_string();
                        }
                    } else if line.contains("_NET_WM_NAME") {
                        if let Some(start) = line.find('"') {
                            if let Some(end) = line.rfind('"') {
                                if end > start + 1 {
                                    window_title = line[start+1..end].to_string();
                                }
                            }
                        }
                    }
                }
                
                if !app_name.is_empty() && !window_title.is_empty() {
                    if debug {
                        println!("~=~ xprop detected: {} - {}", app_name, window_title);
                    }
                    return Some((app_name, window_title));
                }
            }
        }
        
        None
    }

    fn try_process_scanning_fallback(debug: bool) -> Option<(String, String)> {
        // Last resort: scan for common GUI processes
        let gui_processes = vec![
            "firefox", "chrome", "chromium", "code", "cursor", "vim", "nvim",
            "emacs", "atom", "sublime", "kate", "gedit", "terminal", "konsole",
            "gnome-terminal", "xterm", "kitty", "alacritty", "hyper"
        ];
        
        for process in gui_processes {
            if let Ok(output) = Command::new("pgrep")
                .args(&["-f", process])
                .output() {
                
                if output.status.success() && !output.stdout.is_empty() {
                    let pids = String::from_utf8_lossy(&output.stdout);
                    if let Some(pid) = pids.lines().next() {
                        if debug {
                            println!("~=~ Process fallback detected: {} (PID: {})", process, pid);
                        }
                        return Some((process.to_string(), format!("{} window", process)));
                    }
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