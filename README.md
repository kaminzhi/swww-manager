# SWWW Manager

Advanced wallpaper manager for Hyprland using swww with multi-monitor support, auto-switching, and event-driven architecture.

## Features

- **Multi-monitor profiles** - Different wallpaper sets for different display configurations
- **Event-driven** - Uses Hyprland socket2 for instant monitor detection (zero polling)
- **Socket activation** - On-demand startup via systemd (0 MB when idle)
- **Auto-switching** - Automatically change wallpapers at configured intervals
- **Transition effects** - Support for all swww transition types
- **IPC-based** - Pure socket communication, no subprocess spawning for monitor detection
- **Rust** - Fast, reliable, and memory-safe

## Quick Start

### Prerequisites

- Hyprland
- swww
- Rust toolchain (for building)

### Installation

```bash
# Clone repository
git clone https://github.com/yourusername/swww-manager
cd swww-manager

# Run installation script (interactive)
chmod +x install.sh
./install.sh

```

During install you'll be asked how to start swww-manager:

- systemd user units (recommended; socket activation)
- Hyprland exec-once (no systemd)
- Sway exec_always (no systemd)
- None (configure manually)

### Manual Installation

```bash
# Build
cargo build --release

# Install binary
sudo cp target/release/swww-manager /usr/local/bin/

# Option A: systemd (socket activation)
mkdir -p ~/.config/systemd/user
cp systemd/* ~/.config/systemd/user/
systemctl --user daemon-reload
swww-manager init
systemctl --user enable --now swww-manager.socket
systemctl --user enable --now swww-monitor.service  # optional

# Option B: Hyprland (no systemd)
# In ~/.config/hypr/hyprland.conf
#   exec-once = swww init
#   exec-once = swww-manager serve
#   exec-once = swww-manager monitor-events

# Option C: Sway (no systemd)
# In ~/.config/sway/config
#   exec_always swww init
#   exec_always swww-manager serve
#   exec_always swww-manager monitor-events
```

## Usage

### Basic Commands

```bash
# Switch wallpaper
swww-manager switch

# List all profiles
swww-manager list

# Switch to specific profile
swww-manager profile gaming

# Show status
swww-manager status

# Show monitors
swww-manager monitors

# Auto-detect and switch profile
swww-manager detect

# Enable auto-switch (5 min interval)
swww-manager auto on --interval 300
```

### Service Management (systemd option)

```bash
# Check socket status
systemctl --user status swww-manager.socket

# Check monitor service
systemctl --user status swww-monitor.service

# View logs
journalctl --user -u swww-manager.service -f
journalctl --user -u swww-monitor.service -f

# Restart services
systemctl --user restart swww-manager.socket
systemctl --user restart swww-monitor.service
```

## Configuration

Edit `~/.config/swww-manager/config.toml`:

```toml
current_profile = "default"

[auto_switch]
enabled = false
interval = 300
mode = "random"  # or "sequential"

[monitor_detection]
enabled = true

[profiles.default]
monitors = ["*"]
wallpaper_dirs = ["~/Pictures/Wallpapers"]
transition = "wipe"
transition_duration = 2

[profiles.gaming]
monitors = ["DP-1", "HDMI-A-1"]
wallpaper_dirs = ["~/Pictures/Gaming"]
transition = "fade"
transition_duration = 3
```

Hot reload: editing `~/.config/swww-manager/config.toml` is detected automatically; the server will reload config and run one detect/refresh.

See `config.sample.toml` for more examples.

## Architecture

```
┌─────────────────┐
│  Hyprland       │
│  socket2.sock   │  Monitor events
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│ swww-monitor    │  Event listener
│   .service      │  (always running)
└────────┬────────┘
         │
         ↓  Trigger profile switch
┌─────────────────┐
│ swww-manager    │
│   .socket       │  Socket activation
└────────┬────────┘
         │
         ↓  On demand
┌─────────────────┐
│ swww-manager    │  Request handler
│   serve         │  (auto-start/exit)
└─────────────────┘
```

Quick checks:

```bash
# Check if in Hyprland
echo $HYPRLAND_INSTANCE_SIGNATURE

# Check socket
ls -la $XDG_RUNTIME_DIR/swww-manager.sock

# Test IPC
swww-manager status

# Check swww daemon
swww query
```

## Uninstallation

```bash
./uninstall.sh
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- serve

# Format code
cargo fmt

# Lint
cargo clippy
```

## Acknowledgments

- [Hyprland](https://github.com/hyprwm/Hyprland) - Amazing Wayland compositor
- [swww](https://github.com/Horus645/swww) - Wallpaper daemon
