# FocusDebt - Session-Based Focus Tracking CLI 🧠

Cross-platform CLI tool that tracks developer focus and context switching with individual session management. Each work session is tracked independently with user-defined names.

## 🚀 Quick Start

```bash
# Start a new focus session (with interactive prompt)
cargo run -- start

# Stop current session and view summary  
cargo run -- stop

# View recent sessions
cargo run -- stats

# List all past sessions
cargo run -- sessions list
```

## 📚 Complete Command Reference

### 🔄 Session Control

#### Start Session (Interactive)
```bash
cargo run -- start
```
**Interactive Prompt:**
```
🚀 Starting FocusDebt Session Tracker

📝 Please name this focus session:
   Examples: "Morning coding", "Bug fixes", "Learning Rust", "Meeting prep"
   
Session name: ▌
```

#### Stop Session
```bash
cargo run -- stop
```
- Stops daemon and shows session summary
- Saves all session data to database

### 📊 Session Management

#### List Sessions
```bash
# List recent sessions (last 20)
cargo run -- sessions list

# List specific number of sessions
cargo run -- sessions list --last 10

# List sessions for specific date
cargo run -- sessions list --date 2024-01-15
```

**Output Example:**
```
📋 Sessions:
1. "Morning coding"        Jan 15, 09:00-11:30  2h 30m  Focus: 85%  
2. "Bug fixes"            Jan 15, 14:00-16:15  2h 15m  Focus: 72%
3. "Learning Rust"        Jan 14, 19:00-21:00  2h 00m  Focus: 91%
```

#### View Session Details
```bash
# View by session name
cargo run -- sessions show "Morning coding"

# View by session ID
cargo run -- sessions show 1
```

**Output Example:**
```
📊 Session Report: "Morning coding"
🕐 Duration: Jan 15, 09:00-11:30 (2h 30m)
📈 Focus Efficiency: 85%

🏆 Apps Used:
  1. cursor - 1h 45m (Focus)
  2. firefox - 25m (Focus)  
  3. discord - 15m (Distraction)

🔄 Context Switches: 12
⏰ Avg Recovery Time: 45s
```

### 📈 Statistics & Reports

#### Recent Sessions Summary
```bash
cargo run -- stats
```

**Output Example:**
```
📊 Recent Focus Sessions Summary:

Today (Jan 15):
├─ "Morning coding"     09:00-11:30  2h 30m  (85% focus)
└─ "Bug fixes"         14:00-16:15  2h 15m  (72% focus)

Yesterday (Jan 14):
├─ "Learning Rust"     19:00-21:00  2h 00m  (91% focus)  
└─ "Meeting prep"      08:30-09:00  30m     (65% focus)

Use 'focusdebt sessions show <name>' for detailed session reports
```

#### Weekly Statistics
```bash
cargo run -- stats --weekly
```

#### ASCII Art Report (Shareable)
```bash
cargo run -- share
```

### 📤 Data Export

#### Export Session Data
```bash
# Export as JSON (default)
cargo run -- export

# Export as CSV
cargo run -- export --format csv

# Export as HTML
cargo run -- export --format html

# Export with date range
cargo run -- export --format csv --start-date 2024-01-01 --end-date 2024-01-31

# Export to specific file
cargo run -- export --format json --output my-focus-data.json
```

### 🎯 Focus App Management

#### Manage Focus Apps
```bash
# Add apps to focus list (productive apps)
cargo run -- focusapp add cursor
cargo run -- focusapp add code
cargo run -- focusapp add kitty

# Remove app from focus list
cargo run -- focusapp remove chrome

# List all focus vs distraction apps
cargo run -- focusapp list
```

### ⚙️ Configuration

#### Configuration Management
```bash
# Show current configuration
cargo run -- config show

# Set configuration values
cargo run -- config set deep_focus_threshold 900
cargo run -- config set tracking_interval 2000

# Reset configuration to defaults
cargo run -- config reset
```

### 🗄️ Database Management

#### Database Operations
```bash
# Show database statistics
cargo run -- database stats

# Clean up invalid/broken sessions (>24h, <1s, incomplete)
cargo run -- database cleanup

# Clear ALL data completely
cargo run -- database clear

# Keep only recent data (last N days)
cargo run -- database clear-old --days 7
cargo run -- database clear-old --days 30

# Optimize/vacuum database
cargo run -- database optimize
```

### 🔍 Debugging

#### Window Detection Debug
```bash
cargo run -- debug
```

## 🎛️ Command Categories

- **Session Control**: `start`, `stop`
- **Session Management**: `sessions list`, `sessions show`
- **Statistics**: `stats`, `stats --weekly`, `share`
- **Data Export**: `export` with various formats
- **Configuration**: `config` commands, `focusapp` commands
- **Database**: `database` commands
- **Debugging**: `debug`

## 📋 Example Workflows

### Fresh Start
```bash
cargo run -- database clear
cargo run -- focusapp add cursor
cargo run -- focusapp add code
cargo run -- start
# Enter session name when prompted
```

### Daily Routine
```bash
cargo run -- start          # Morning (enter session name)
# Work throughout the day...
cargo run -- stop           # Evening
cargo run -- stats          # Check daily report
```

### Weekly Review
```bash
cargo run -- stats --weekly
cargo run -- share          # Get shareable report
cargo run -- export --format csv --output weekly-report.csv
```

### Session Analysis
```bash
cargo run -- sessions list
cargo run -- sessions show "Morning coding"
cargo run -- sessions list --date 2024-01-15
```

### Data Maintenance
```bash
cargo run -- database stats
cargo run -- database cleanup
cargo run -- database clear-old --days 30
cargo run -- database optimize
```

## 🏗️ Architecture

- **Language**: Rust (Edition 2021)
- **CLI**: clap crate with derive features
- **Storage**: SQLite with session-based tracking
- **Config**: TOML-based configuration system
- **Cross-platform**: Linux (X11/Wayland), macOS, Windows
- **Export**: JSON, CSV, HTML with styling
- **Security**: Input validation, path sanitization, safe process handling

## 📁 Data Storage

- **Configuration**: `~/.config/focusdebt/config.toml`
- **Database**: `~/.local/share/focusdebt/focusdebt.db`
- **Session Data**: Individual sessions with names, not aggregated

## 🔧 Installation

### Prerequisites

#### Linux
```bash
# Install xdotool for window tracking
sudo pacman -S xdotool  # Arch Linux
# or
sudo apt install xdotool  # Ubuntu/Debian
```

#### macOS
No additional dependencies required - uses built-in AppleScript.

#### Windows
No additional dependencies required - uses PowerShell and Windows API.

### Build from Source

```bash
git clone <repository>
cd focusdebt
cargo build --release

# Run with cargo
cargo run -- start
```

### Install System-wide

```bash
cargo install --path .
```

## ⚙️ Configuration

FocusDebt uses a TOML configuration file located at:
- **Linux/macOS**: `~/.config/focusdebt/config.toml`
- **Windows**: `%APPDATA%\focusdebt\config.toml`

### Configuration Options

```toml
# Tracking intervals (in milliseconds)
tracking_interval_ms = 1000
save_interval_ms = 30000

# Deep focus threshold (in minutes)
deep_focus_threshold_minutes = 30

# Focus applications
focus_apps = ["code", "vim", "emacs", "sublime"]

# Ignored applications
ignored_apps = ["system", "desktop"]

# Logging
log_level = "info"

# Notifications
[notifications]
enabled = false
interval_minutes = 60
focus_reminders = false
break_reminders = false

# Export settings
[export]
auto_export = false
format = "json"
export_path = "~/Documents/focusdebt_exports"
```

## 🎯 Key Features

- ✅ **Session-Based Tracking**: Each session independently named and tracked
- ✅ **Interactive Session Starting**: Prompted naming with examples
- ✅ **Cross-Platform Window Detection**: 9 different detection methods
- ✅ **Focus vs Distraction Classification**: App-based categorization
- ✅ **Context Switch Analysis**: Recovery time tracking
- ✅ **Multiple Export Formats**: JSON, CSV, HTML
- ✅ **Database Management**: Cleanup, optimization, retention
- ✅ **Session History**: List and view past sessions individually
- ✅ **Focus Efficiency Calculation**: Per-session and overall metrics

## 📊 Understanding Your Data

### Focus Sessions
- **Duration**: Time spent in focus applications per session
- **Deep Focus**: Sessions longer than your threshold
- **Efficiency**: Percentage of time spent in focus vs distraction per session

### Context Switches
- **Frequency**: How often you switch between applications during a session
- **Recovery Time**: Time lost when switching back to focus work
- **Patterns**: Identify your most distracting applications per session

### Session-Based Metrics
- **Focus Efficiency**: `(focus_time / total_time) * 100` per session
- **Session Comparison**: Compare different named sessions
- **Individual Tracking**: Each session tracked separately, not aggregated

## 🔧 Troubleshooting

### Common Issues

#### "Required dependencies not found"
- **Linux**: Install xdotool: `sudo pacman -S xdotool`
- **macOS**: Should work out of the box
- **Windows**: Should work out of the box

#### "Failed to initialize database"
- Check write permissions in data directory
- Ensure sufficient disk space
- Try resetting configuration: `cargo run -- config reset`

#### "No active window detected"
- **Linux**: Ensure xdotool is installed and working
- **macOS**: Grant accessibility permissions to terminal
- **Windows**: Run as administrator if needed

### Debug Mode
```bash
cargo run -- debug
```

## 🛡️ Security Features

- **Input Validation**: All user inputs are validated and sanitized
- **Path Traversal Protection**: Secure file path handling
- **Command Injection Prevention**: Safe process management
- **PID Validation**: Secure process ID handling
- **Local Storage**: All data stored locally, no external servers

## 📄 License

This project is licensed under the MIT License.

---

**FocusDebt** - Track your focus sessions independently. 🎯