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

# Stop services
print_info "Stopping services..."
systemctl --user stop swww-manager.socket 2>/dev/null || true
systemctl --user stop swww-monitor.service 2>/dev/null || true
systemctl --user stop swww-timer.timer 2>/dev/null || true

# Disable services
print_info "Disabling services..."
systemctl --user disable swww-manager.socket 2>/dev/null || true
systemctl --user disable swww-monitor.service 2>/dev/null || true
systemctl --user disable swww-timer.timer 2>/dev/null || true

# Remove systemd units
print_info "Removing systemd units..."
rm -f ~/.config/systemd/user/swww-manager.socket
rm -f ~/.config/systemd/user/swww-manager@.service
rm -f ~/.config/systemd/user/swww-monitor.service
rm -f ~/.config/systemd/user/swww-timer.timer
rm -f ~/.config/systemd/user/swww-timer.service

systemctl --user daemon-reload
print_success "Systemd units removed"

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
