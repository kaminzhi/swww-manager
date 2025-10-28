#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

echo -e "${RED}"
cat <<"EOF"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║            SWWW Manager Uninstallation Script                 ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}"

print_warning "This will remove SWWW Manager from your system"
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Aborted"
  exit 0
fi

# Helper: stop/disable unit if it exists
stop_disable_unit() {
  local unit="$1"
  if systemctl --user list-unit-files | grep -q "^${unit}\\>"; then
    systemctl --user stop "$unit" 2>/dev/null || true
    systemctl --user disable "$unit" 2>/dev/null || true
    print_success "Stopped/disabled $unit"
  else
    print_info "$unit not installed or already removed"
  fi
}

# Auto-detect systemd usage (units installed under user dir)
SYSTEMD_USED=false
if ls ~/.config/systemd/user 1>/dev/null 2>&1; then
  if ls ~/.config/systemd/user | grep -Eq "^swww-(manager|monitor|timer)"; then
    SYSTEMD_USED=true
  fi
fi

if [ "$SYSTEMD_USED" = true ]; then
  print_info "Detected systemd user units. Cleaning them up..."

  # Stop/disable known units if present
  stop_disable_unit swww-manager.socket
  stop_disable_unit swww-manager.service
  stop_disable_unit swww-monitor.service
  stop_disable_unit swww-timer.timer
  stop_disable_unit swww-timer.service

  # Remove unit files if they exist
  print_info "Removing systemd unit files..."
  rm -f ~/.config/systemd/user/swww-manager.socket
  rm -f ~/.config/systemd/user/swww-manager.service
  rm -f ~/.config/systemd/user/swww-manager@.service
  rm -f ~/.config/systemd/user/swww-monitor.service
  rm -f ~/.config/systemd/user/swww-timer.timer
  rm -f ~/.config/systemd/user/swww-timer.service

  systemctl --user daemon-reload || true
  print_success "Systemd units removed"
else
  print_info "No systemd user units detected. Skipping systemd cleanup."
fi

# Remove runtime socket if any (for foreground/manual runs)
rm -f "/run/user/$UID/swww-manager.sock" 2>/dev/null || true

# Remove binary
print_info "Removing binary..."
if sudo rm -f /usr/local/bin/swww-manager; then
  print_success "Binary removed"
else
  print_warning "Failed to remove binary"
fi

# Ask about config
echo
read -p "Remove configuration files? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  rm -rf ~/.config/swww-manager
  print_success "Configuration removed"
else
  print_info "Configuration kept at ~/.config/swww-manager"
fi

# Ask about wallpapers
echo
read -p "Remove wallpapers directory? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  rm -rf ~/Pictures/Wallpapers
  print_success "Wallpapers directory removed"
else
  print_info "Wallpapers kept at ~/Pictures/Wallpapers"
fi

echo
echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                  Uninstallation Complete!                      ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
echo
