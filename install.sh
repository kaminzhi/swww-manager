#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Functions
print_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
  echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

check_dependencies() {
  print_info "Checking dependencies..."

  local missing_deps=()

  # Check for required tools
  if ! command -v cargo &>/dev/null; then
    missing_deps+=("cargo (Rust toolchain)")
  fi

  if ! command -v swww &>/dev/null; then
    print_warning "swww not found. Please install it from: https://github.com/Horus645/swww"
  fi

  if ! command -v hyprctl &>/dev/null; then
    print_warning "hyprctl not found. This tool requires Hyprland."
  fi

  if [ ${#missing_deps[@]} -ne 0 ]; then
    print_error "Missing required dependencies:"
    for dep in "${missing_deps[@]}"; do
      echo "  - $dep"
    done
    exit 1
  fi

  print_success "All required dependencies found"
}

build_project() {
  print_info "Building swww-manager..."

  if ! cargo build --release; then
    print_error "Build failed"
    exit 1
  fi

  print_success "Build completed"
}

install_binary() {
  print_info "Installing binary..."

  local binary_path="target/release/swww-manager"
  local install_path="/usr/local/bin/swww-manager"

  if [ ! -f "$binary_path" ]; then
    print_error "Binary not found at $binary_path"
    exit 1
  fi

  # Try to install with sudo
  if sudo cp "$binary_path" "$install_path"; then
    sudo chmod +x "$install_path"
    print_success "Binary installed to $install_path"
  else
    print_error "Failed to install binary (permission denied?)"
    exit 1
  fi
}

install_systemd_units() {
  print_info "Installing systemd units..."

  local systemd_dir="$HOME/.config/systemd/user"
  mkdir -p "$systemd_dir"

  # Copy all systemd files
  for file in systemd/*.{socket,service,timer}; do
    if [ -f "$file" ]; then
      cp "$file" "$systemd_dir/"
      print_success "Installed $(basename $file)"
    fi
  done

  # Reload systemd
  systemctl --user daemon-reload
  print_success "Systemd units reloaded"
}

generate_config() {
  print_info "Generating configuration..."

  local config_dir="$HOME/.config/swww-manager"
  local config_file="$config_dir/config.toml"

  if [ -f "$config_file" ]; then
    print_warning "Config file already exists at $config_file"
    read -p "Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      print_info "Keeping existing config"
      return
    fi
  fi

  # Create config directory
  mkdir -p "$config_dir"

  # Copy example config
  if [ -f "config.example.toml" ]; then
    cp config.example.toml "$config_file"
    print_success "Config generated at $config_file"
  else
    # Generate using binary
    if /usr/local/bin/swww-manager init; then
      print_success "Config generated at $config_file"
    else
      print_error "Failed to generate config"
      exit 1
    fi
  fi
}

create_wallpaper_dirs() {
  print_info "Creating wallpaper directories..."

  local wallpaper_dir="$HOME/Pictures/Wallpapers"

  if [ ! -d "$wallpaper_dir" ]; then
    mkdir -p "$wallpaper_dir"
    print_success "Created $wallpaper_dir"
  else
    print_info "Wallpaper directory already exists"
  fi
}

enable_services() {
  print_info "Enabling systemd services..."

  # Enable socket (required)
  if systemctl --user enable swww-manager.socket; then
    print_success "Enabled swww-manager.socket"
  else
    print_warning "Failed to enable swww-manager.socket"
  fi

  # Ask about monitor service
  echo
  read -p "Enable monitor detection service? (Y/n): " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    if systemctl --user enable swww-monitor.service; then
      print_success "Enabled swww-monitor.service"
    fi
  fi

  # Ask about timer
  echo
  read -p "Enable auto-switch timer? (y/N): " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    if systemctl --user enable swww-timer.timer; then
      print_success "Enabled swww-timer.timer"
    fi
  fi
}

start_services() {
  print_info "Starting services..."

  # Start socket
  if systemctl --user start swww-manager.socket; then
    print_success "Started swww-manager.socket"
  fi

  # Start monitor if enabled
  if systemctl --user is-enabled swww-monitor.service &>/dev/null; then
    if systemctl --user start swww-monitor.service; then
      print_success "Started swww-monitor.service"
    fi
  fi

  # Start timer if enabled
  if systemctl --user is-enabled swww-timer.timer &>/dev/null; then
    if systemctl --user start swww-timer.timer; then
      print_success "Started swww-timer.timer"
    fi
  fi
}

show_completion() {
  echo
  echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
  echo -e "${GREEN}║                    Installation Complete!                     ║${NC}"
  echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
  echo
  print_info "Configuration file: $HOME/.config/swww-manager/config.toml"
  print_info "Wallpaper directory: $HOME/Pictures/Wallpapers"
  echo
  print_info "Quick start commands:"
  echo "  swww-manager switch              # Switch wallpaper"
  echo "  swww-manager list                # List profiles"
  echo "  swww-manager status              # Show status"
  echo "  swww-manager monitors            # Show monitors"
  echo
  print_info "Service management:"
  echo "  systemctl --user status swww-manager.socket"
  echo "  systemctl --user status swww-monitor.service"
  echo "  journalctl --user -u swww-manager@.service -f"
  echo
  print_info "For more information, see: README.md"
  echo
}

# Main installation flow
main() {
  echo -e "${BLUE}"
  cat <<"EOF"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║              SWWW Manager Installation Script                 ║
║                                                               ║
║          Hyprland Wallpaper Manager with IPC & Events         ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
EOF
  echo -e "${NC}"

  check_dependencies
  build_project
  install_binary
  install_systemd_units
  generate_config
  create_wallpaper_dirs
  enable_services
  start_services
  show_completion
}

# Run installation
main
