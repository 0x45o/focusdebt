use std::process::Command;
use std::time::Duration;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

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
    timestamp.format("%H:%M:%S").to_string()
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
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find data directory"))?;
    
    // Validate path to prevent path traversal
    if !is_safe_path(&data_dir) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid data directory path"
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
    path_str.starts_with('/') || path_str.starts_with("C:\\")
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
                "Invalid PID file path"
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