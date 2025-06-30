# FocusDebt 🧠

A cross-platform CLI tool that tracks developer focus and context switching to help you understand your productivity patterns and reduce focus debt.

## 🚀 Features

- **🔄 Context Switch Tracking**: Monitor how often you switch between applications
- **⏱️ Focus Time Measurement**: Track time spent in focus vs distraction apps
- **🧠 Deep Focus Sessions**: Identify sessions of 30+ minutes of uninterrupted focus
- **⏰ Recovery Time Analysis**: Measure how long it takes to get back into focus
- **📊 Rich Statistics**: Daily and weekly breakdowns with detailed metrics
- **🎨 Shareable Reports**: Generate beautiful ASCII art reports for social media
- **🖥️ Cross-Platform**: Works on Linux, macOS, and Windows
- **💾 Persistent Storage**: SQLite database stores all your data locally
- **Data Export**: Export data in JSON, CSV, and HTML formats
- **Configuration Management**: Customizable settings via config files

## 🔒 Security Features

- **Input Validation**: All user inputs are validated and sanitized
- **Path Traversal Protection**: Secure file path handling
- **Command Injection Prevention**: Safe process management
- **PID Validation**: Secure process ID handling
- **Configurable Permissions**: Granular access control

## 📦 Installation

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
git clone https://github.com/yourusername/focusdebt.git
cd focusdebt
cargo build --release
```

### Install System-wide

```bash
cargo install --path .
```

## 🎯 Usage

### Basic Commands

```bash
# Start tracking focus
focusdebt start

# Stop tracking and show summary
focusdebt stop

# View daily statistics
focusdebt stats

# View weekly statistics
focusdebt stats --weekly

# Generate shareable report
focusdebt share
```

### Data Export

```bash
# Export today's data as JSON
focusdebt export

# Export specific date range as CSV
focusdebt export --format csv --start-date 2024-01-01 --end-date 2024-01-31

# Export as HTML report
focusdebt export --format html --output report.html
```

### Focus App Management

```bash
# Add an application to focus list
focusdebt focusapp add "code"

# Remove from focus list
focusdebt focusapp remove "discord"

# List current focus apps
focusdebt focusapp list
```

### Configuration

```bash
# Show current configuration
focusdebt config show

# Set configuration values
focusdebt config set tracking_interval_ms 2000
focusdebt config set deep_focus_threshold_minutes 45

# Reset to defaults
focusdebt config reset
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

## 📊 Understanding Your Data

### Focus Sessions
- **Duration**: Time spent in focus applications
- **Deep Focus**: Sessions longer than your threshold
- **Efficiency**: Percentage of time spent in focus vs distraction

### Context Switches
- **Frequency**: How often you switch between applications
- **Recovery Time**: Time lost when switching back to focus work
- **Patterns**: Identify your most distracting applications

### Metrics Explained
- **Focus Efficiency**: `(focus_time / total_time) * 100`
- **Deep Focus Sessions**: Extended periods of focused work
- **Average Recovery Time**: Time to regain focus after switching

## 🔧 Troubleshooting

### Common Issues

#### "Required dependencies not found"
- **Linux**: Install xdotool: `sudo pacman -S xdotool`
- **macOS**: Should work out of the box
- **Windows**: Should work out of the box

#### "Failed to initialize database"
- Check write permissions in data directory
- Ensure sufficient disk space
- Try resetting configuration: `focusdebt config reset`

#### "No active window detected"
- **Linux**: Ensure xdotool is installed and working
- **macOS**: Grant accessibility permissions to terminal
- **Windows**: Run as administrator if needed

### Debug Mode

```bash
# Run with verbose output
RUST_LOG=debug focusdebt start
```

## 🛡️ Security Considerations

### Data Privacy
- All data is stored locally on your machine
- No data is sent to external servers
- Database files are stored in user data directory

### Permissions
- **Linux**: Requires xdotool for window tracking
- **macOS**: May require accessibility permissions
- **Windows**: May require administrator privileges

### File Locations
- **Data**: `~/.local/share/focusdebt/` (Linux/macOS) or `%LOCALAPPDATA%\focusdebt\` (Windows)
- **Config**: `~/.config/focusdebt/` (Linux/macOS) or `%APPDATA%\focusdebt\` (Windows)
- **Logs**: Same as data directory

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes
4. Run tests: `cargo test`
5. Commit your changes: `git commit -am 'Add feature'`
6. Push to the branch: `git push origin feature-name`
7. Submit a pull request

### Development Setup

```bash
# Clone and setup
git clone https://github.com/yourusername/focusdebt.git
cd focusdebt

# Install dependencies
cargo build

# Run tests
cargo test

# Run linter
cargo clippy

# Format code
cargo fmt
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by productivity tracking tools like RescueTime and Toggl
- Built with Rust for performance and cross-platform compatibility
- Uses SQLite for reliable local data storage

## 📈 Roadmap

- [ ] Web dashboard for data visualization
- [ ] Integration with calendar applications
- [ ] Focus streak tracking
- [ ] Pomodoro timer integration
- [ ] Team productivity insights
- [ ] API for third-party integrations
- [ ] Mobile companion app
- [ ] Cloud sync (optional)

---

**FocusDebt** - Track your focus, reduce your debt. 🎯 