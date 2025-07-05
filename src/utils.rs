use std::process::Command;
use std::time::Duration;
use chrono::{DateTime, Utc, Local};
use std::path::PathBuf;
use regex::Regex;
use url::Url;
use std::collections::HashSet;

pub fn check_dependencies() -> bool {
    // Check if xdotool is available on Linux
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("which").arg("xdotool").output();
        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        true // For now, assume dependencies are available on other platforms
    }
}

pub fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.with_timezone(&Local).format("%H:%M:%S").to_string()
}

pub fn format_timestamp_local(timestamp: DateTime<Utc>) -> String {
    timestamp.with_timezone(&Local).format("%H:%M").to_string()
}

pub fn format_datetime_local(timestamp: DateTime<Utc>) -> String {
    timestamp.with_timezone(&Local).format("%b %d, %H:%M").to_string()
}

pub fn format_duration_short(duration: Duration) -> String {
    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;
    
    if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

pub fn get_data_directory() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("focusdebt"))
}

pub fn ensure_data_directory() -> std::io::Result<PathBuf> {
    let data_dir = get_data_directory()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "❌ Could not find data directory"))?;
    
    // Validate path to prevent path traversal
    if !is_safe_path(&data_dir) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "❌ Invalid data directory path"
        ));
    }
    
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir)
}

fn is_safe_path(path: &PathBuf) -> bool {
    // Check for path traversal attempts
    let path_str = path.to_string_lossy();
    !path_str.contains("..") && 
    !path_str.contains("//") && 
    (path_str.starts_with('/') || path_str.starts_with("C:\\") || path.is_absolute())
}

pub fn is_daemon_running() -> bool {
    // Check if there's a PID file or process running
    if let Some(data_dir) = get_data_directory() {
        let pid_file = data_dir.join("focusdebt.pid");
        if pid_file.exists() {
            if let Ok(pid_content) = std::fs::read_to_string(&pid_file) {
                // Validate PID content - should only contain digits
                let pid_content = pid_content.trim();
                if pid_content.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(pid) = pid_content.parse::<u32>() {
                        // Validate PID range (1-999999 is reasonable)
                        if pid > 0 && pid < 1000000 {
                            // Check if process is still running using safe method
                            return check_process_exists(pid);
                        }
                    }
                }
            }
        }
    }
    false
}

fn check_process_exists(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        // Use /proc filesystem instead of kill command
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
    
    #[cfg(target_os = "macos")]
    {
        // Use ps command with safe arguments
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "pid="])
            .output();
        match output {
            Ok(output) => output.status.success() && !output.stdout.is_empty(),
            Err(_) => false,
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Use tasklist with safe arguments
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/FO", "CSV"])
            .output();
        match output {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.contains(&pid.to_string())
            },
            Err(_) => false,
        }
    }
}

pub fn write_pid_file(pid: u32) -> std::io::Result<()> {
    if let Some(data_dir) = get_data_directory() {
        let pid_file = data_dir.join("focusdebt.pid");
        // Validate path before writing
        if !is_safe_path(&pid_file) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "❌ Invalid PID file path"
            ));
        }
        std::fs::write(pid_file, pid.to_string())?;
    }
    Ok(())
}

pub fn remove_pid_file() -> std::io::Result<()> {
    if let Some(data_dir) = get_data_directory() {
        let pid_file = data_dir.join("focusdebt.pid");
        if pid_file.exists() && is_safe_path(&pid_file) {
            std::fs::remove_file(pid_file)?;
        }
    }
    Ok(())
}

pub fn get_current_pid() -> u32 {
    std::process::id()
}

pub fn sleep_ms(milliseconds: u64) {
    std::thread::sleep(Duration::from_millis(milliseconds));
}

pub fn extract_domain_from_title(window_title: &str, app_name: &str) -> Option<String> {
    // Common browser process names
    let browser_apps = [
        "chrome", "firefox", "safari", "edge", "brave", "chromium", "opera", "vivaldi"
    ];
    
    // Check if this is a browser
    let is_browser = browser_apps.iter().any(|&browser| {
        app_name.to_lowercase().contains(browser)
    });
    
    if !is_browser {
        return None;
    }
    
    // Try to extract domain from various title patterns
    let patterns = [
        // "Page Title - Browser Name" -> extract from title
        r"^(.+?)\s*[-–—]\s*(?:Google Chrome|Firefox|Safari|Edge|Brave|Chromium|Opera|Vivaldi)$",
        // "Page Title | Domain" -> extract domain
        r"^(.+?)\s*\|\s*([^|]+)$",
        // "Domain - Page Title" -> extract domain
        r"^([^-\s]+)\s*[-–—]\s*(.+)$",
        // Just try to find a domain-like pattern
        r"([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}",
    ];
    
    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(window_title) {
                if pattern == &patterns[0] {
                    // First pattern: extract from title part
                    let title_part = captures.get(1).unwrap().as_str();
                    return extract_domain_from_text(title_part);
                } else if pattern == &patterns[1] {
                    // Second pattern: extract from domain part
                    let domain_part = captures.get(2).unwrap().as_str();
                    return Some(domain_part.trim().to_lowercase());
                } else if pattern == &patterns[2] {
                    // Third pattern: extract from domain part
                    let domain_part = captures.get(1).unwrap().as_str();
                    return Some(domain_part.trim().to_lowercase());
                } else {
                    // Fourth pattern: direct domain match
                    let domain = captures.get(1).unwrap().as_str();
                    return Some(domain.to_lowercase());
                }
            }
        }
    }
    
    // Fallback: try to extract domain from the entire title
    extract_domain_from_text(window_title)
}

fn extract_domain_from_text(text: &str) -> Option<String> {
    // Look for URL patterns
    let url_pattern = Regex::new(r"https?://([^/\s]+)").ok()?;
    if let Some(captures) = url_pattern.captures(text) {
        return Some(captures.get(1).unwrap().as_str().to_lowercase());
    }
    
    // Look for domain patterns
    let domain_pattern = Regex::new(r"([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}").ok()?;
    if let Some(captures) = domain_pattern.captures(text) {
        return Some(captures.get(1).unwrap().as_str().to_lowercase());
    }
    
    None
}

pub fn get_running_apps() -> Vec<(String, String)> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    
    #[cfg(target_os = "linux")]
    {
        // Try to get running GUI applications using ps
        if let Ok(output) = Command::new("ps")
            .args(&["-eo", "comm,pid"])
            .output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            let gui_apps = [
                // Editors/IDEs
                "code", "firefox", "chrome", "sublime", "atom", "gedit", "kate",
                "vim", "nvim", "neovim", "emacs", "intellij", "pycharm", "webstorm", "clion",
                "eclipse", "netbeans", "android-studio", "vscodium", "cursor",
                // Terminals
                "kitty", "alacritty", "wezterm", "gnome-terminal", "konsole", "xterm", "rxvt",
                // Browsers
                "brave", "chromium", "opera", "safari", "edge", "vivaldi",
            ];
            let skip_patterns = ["crashpad", "gnome-keyring", "at-spi", "dbus", "xdg", "gvfs", "pulseaudio", "pipewire", "systemd", "ibus", "gnome-session", "gnome-shell", "Xorg", "Xwayland", "wayland"];
            
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let comm = parts[0];
                    // Filter for common GUI applications and skip system/crashpad
                    if gui_apps.iter().any(|&app| comm.contains(app)) && !skip_patterns.iter().any(|&skip| comm.contains(skip)) {
                        let friendly_name = get_friendly_app_name(comm);
                        // Deduplicate by friendly name
                        if seen.insert(friendly_name.clone()) {
                            apps.push((friendly_name, comm.to_string()));
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS implementation
        if let Ok(output) = Command::new("ps")
            .args(&["-eo", "comm"])
            .output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let gui_apps = [
                "Code", "Firefox", "Chrome", "Safari", "Terminal", "iTerm2", "Sublime Text", "Atom", "Xcode", "IntelliJ", "PyCharm", "Cursor", "kitty", "Alacritty", "WezTerm"
            ];
            let skip_patterns = ["crashpad", "keychain", "dbus", "systemd", "WindowServer"];
            for line in output_str.lines() {
                let comm = line.trim();
                if gui_apps.iter().any(|&app| comm.contains(app)) && !skip_patterns.iter().any(|&skip| comm.contains(skip)) {
                    let friendly_name = get_friendly_app_name(comm);
                    if seen.insert(friendly_name.clone()) {
                        apps.push((friendly_name, comm.to_string()));
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("tasklist")
            .args(&["/FO", "CSV", "/NH"])
            .output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let gui_apps = [
                "code.exe", "firefox.exe", "chrome.exe", "notepad.exe", "sublime_text.exe", "atom.exe", "intellij64.exe", "cursor.exe", "kitty.exe", "alacritty.exe", "wezterm.exe"
            ];
            let skip_patterns = ["crashpad", "system", "conhost", "svchost"];
            for line in output_str.lines() {
                if let Some(comm) = line.split(',').nth(0) {
                    let comm = comm.trim_matches('"');
                    if gui_apps.iter().any(|&app| comm.contains(app)) && !skip_patterns.iter().any(|&skip| comm.contains(skip)) {
                        let friendly_name = get_friendly_app_name(comm);
                        if seen.insert(friendly_name.clone()) {
                            apps.push((friendly_name, comm.to_string()));
                        }
                    }
                }
            }
        }
    }
    
    apps
}

fn get_friendly_app_name(process_name: &str) -> String {
    let friendly_names = [
        ("code", "Visual Studio Code"),
        ("firefox", "Firefox"),
        ("chrome", "Google Chrome"),
        ("sublime", "Sublime Text"),
        ("atom", "Atom"),
        ("gedit", "Gedit"),
        ("kate", "Kate"),
        ("vim", "Vim"),
        ("emacs", "Emacs"),
        ("intellij", "IntelliJ IDEA"),
        ("pycharm", "PyCharm"),
        ("webstorm", "WebStorm"),
        ("clion", "CLion"),
        ("eclipse", "Eclipse"),
        ("netbeans", "NetBeans"),
        ("android-studio", "Android Studio"),
        ("vscodium", "VSCodium"),
        ("neovim", "Neovim"),
        ("brave", "Brave Browser"),
        ("chromium", "Chromium"),
        ("opera", "Opera"),
        ("safari", "Safari"),
        ("edge", "Microsoft Edge"),
        ("vivaldi", "Vivaldi"),
        ("notepad", "Notepad"),
        ("terminal", "Terminal"),
        ("iterm2", "iTerm2"),
    ];
    
    for (key, friendly) in &friendly_names {
        if process_name.to_lowercase().contains(key) {
            return friendly.to_string();
        }
    }
    
    process_name.to_string()
}

pub fn get_open_browser_tabs() -> Vec<String> {
    let mut tabs = Vec::new();
    
    #[cfg(target_os = "linux")]
    {
        // Try to get all browser windows using xdotool
        if let Ok(output) = Command::new("xdotool")
            .args(&["search", "--name", ".*"])
            .output() {
            
            let window_ids = String::from_utf8_lossy(&output.stdout);
            
            for window_id in window_ids.lines() {
                if let Ok(title_output) = Command::new("xdotool")
                    .args(&["getwindowname", window_id])
                    .output() {
                    
                    if title_output.status.success() {
                        let title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();
                        
                        // Check if this is a browser window by looking for browser suffixes
                        let browser_suffixes = [
                            " - Google Chrome",
                            " - Brave",
                            " - Firefox",
                            " - Chromium",
                            " - Opera",
                            " - Safari",
                            " - Edge",
                            " - Vivaldi"
                        ];
                        
                        for suffix in &browser_suffixes {
                            if title.ends_with(suffix) {
                                // Extract the tab name (remove browser suffix)
                                let tab_name = title.trim_end_matches(suffix).trim();
                                if !tab_name.is_empty() && tab_name != "New Tab" {
                                    tabs.push(tab_name.to_string());
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS implementation using AppleScript
        let script = r#"
        tell application "System Events"
            set browserApps to {"Google Chrome", "Safari", "Firefox", "Brave Browser", "Opera"}
            set allTabs to {}
            
            repeat with browserApp in browserApps
                try
                    tell application browserApp
                        repeat with w in windows
                            try
                                set tabTitle to name of w
                                if tabTitle is not "New Tab" and tabTitle is not "" then
                                    set end of allTabs to tabTitle
                                end if
                            end try
                        end repeat
                    end tell
                end try
            end repeat
            
            return allTabs
        end tell
        "#;
        
        if let Ok(output) = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output() {
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    let tab = line.trim();
                    if !tab.is_empty() && tab != "New Tab" {
                        tabs.push(tab.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows implementation using PowerShell
        let script = r#"
        $browsers = @("chrome", "firefox", "msedge", "brave")
        $tabs = @()
        
        foreach ($browser in $browsers) {
            $processes = Get-Process -Name $browser -ErrorAction SilentlyContinue
            foreach ($process in $processes) {
                $windows = Get-Process -Id $process.Id | Where-Object {$_.MainWindowTitle -ne ""}
                foreach ($window in $windows) {
                    $title = $window.MainWindowTitle
                    if ($title -and $title -ne "New Tab") {
                        $tabs += $title
                    }
                }
            }
        }
        
        $tabs | Sort-Object -Unique
        "#;
        
        if let Ok(output) = Command::new("powershell")
            .args(&["-Command", script])
            .output() {
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    let tab = line.trim();
                    if !tab.is_empty() && tab != "New Tab" {
                        tabs.push(tab.to_string());
                    }
                }
            }
        }
    }
    
    // Remove duplicates and sort
    tabs.sort();
    tabs.dedup();
    tabs
} 