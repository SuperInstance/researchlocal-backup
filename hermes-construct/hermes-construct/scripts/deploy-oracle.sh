#!/usr/bin/env bash
#
# deploy-oracle.sh — One-command Oracle Cloud ARM (Ampere A1) deployment.
# Runs ON the Oracle instance. Builds from source, installs as systemd service.
#
# Usage: ./deploy-oracle.sh [--skip-build] [--help]
#
set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
die()   { echo -e "${RED}[FATAL]${NC} $*" >&2; exit 1; }

# ── Help ────────────────────────────────────────────────────────────────────
usage() {
cat <<'EOF'
Usage: deploy-oracle.sh [OPTIONS]

One-command Oracle Cloud ARM deployment for hermes-construct.

Options:
  --skip-build    Skip cargo build (use existing binary)
  --help          Show this help message

Requirements:
  - Oracle Cloud ARM (Ampere A1) instance
  - Internet access (for cargo build)
  - sudo privileges

This script is idempotent — safe to re-run.
EOF
exit 0
}

SKIP_BUILD=false
for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=true ;;
    --help|-h)    usage ;;
    *) die "Unknown argument: $arg. Use --help." ;;
  esac
done

# ── Constants ───────────────────────────────────────────────────────────────
BIN_NAME="hermes-construct"
INSTALL_DIR="/opt/hermes-construct"
BIN_DIR="${INSTALL_DIR}/bin"
STATE_DIR="/var/lib/hermes-construct"
ENV_FILE="/etc/hermes-construct/hermes.env"
SERVICE_NAME="hermes-construct"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
TARGET="aarch64-unknown-linux-gnu"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Step 1: Architecture check ──────────────────────────────────────────────
ARCH="$(uname -m)"
info "Detected architecture: $ARCH"
if [[ "$ARCH" != "aarch64" ]]; then
  warn "This script is designed for aarch64 (ARM64). Detected: $ARCH"
  warn "Continuing anyway — build may fail if cross-compilation isn't set up."
fi

# ── Step 2: Install system dependencies ─────────────────────────────────────
info "Checking system dependencies..."

if ! command -v sqlite3 &>/dev/null; then
  info "Installing SQLite3..."
  if command -v apt-get &>/dev/null; then
    sudo apt-get update -qq && sudo apt-get install -y -qq sqlite3 libsqlite3-dev
  elif command -v dnf &>/dev/null; then
    sudo dnf install -y sqlite sqlite-devel
  elif command -v yum &>/dev/null; then
    sudo yum install -y sqlite sqlite-devel
  else
    die "No supported package manager found. Install sqlite3 manually."
  fi
  ok "SQLite3 installed: $(sqlite3 --version 2>&1 | head -1)"
else
  ok "SQLite3 already installed: $(sqlite3 --version 2>&1 | head -1)"
fi

# Build essentials
for pkg in gcc make pkg-config; do
  if ! command -v "$pkg" &>/dev/null; then
    info "Installing $pkg..."
    sudo apt-get install -y -qq "$pkg" 2>/dev/null || true
  fi
done

# ── Step 3: Install Rust if needed ──────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
  ok "Rust installed: $(rustc --version)"
else
  ok "Rust already installed: $(rustc --version)"
fi

# Ensure aarch64 target is available (for on-ARM native build this is automatic)
rustup target add "$TARGET" 2>/dev/null || true

# ── Step 4: Build release binary ────────────────────────────────────────────
cd "$REPO_ROOT"

if [[ "$SKIP_BUILD" == true ]]; then
  info "Skipping build (--skip-build)"
else
  info "Building release binary (target: $TARGET)..."
  BUILD_TARGET="$TARGET"

  # If we're already on aarch64, build natively for best optimization
  if [[ "$ARCH" == "aarch64" ]]; then
    info "Native ARM build — using host target"
    cargo build --release
    BIN_PATH="target/release/$BIN_NAME"
  else
    cargo build --release --target "$BUILD_TARGET"
    BIN_PATH="target/${BUILD_TARGET}/release/$BIN_NAME"
  fi

  if [[ ! -f "$BIN_PATH" ]]; then
    die "Build failed — binary not found at $BIN_PATH"
  fi
  ok "Build complete: $(du -h "$BIN_PATH" | cut -f1)"
fi

# Find the binary
if [[ "$ARCH" == "aarch64" ]]; then
  BIN_PATH="${REPO_ROOT}/target/release/${BIN_NAME}"
else
  BIN_PATH="${REPO_ROOT}/target/${TARGET}/release/${BIN_NAME}"
fi
[[ -f "$BIN_PATH" ]] || die "Binary not found at $BIN_PATH"

# ── Step 5: Create hermes user ──────────────────────────────────────────────
if id "hermes" &>/dev/null; then
  ok "User 'hermes' already exists"
else
  info "Creating system user 'hermes'..."
  sudo useradd --system --no-create-home --shell /usr/sbin/nologin hermes
  ok "User 'hermes' created"
fi

# ── Step 6: Install binary and assets ───────────────────────────────────────
info "Installing to $INSTALL_DIR..."
sudo mkdir -p "$BIN_DIR" "$INSTALL_DIR/rooms" "$INSTALL_DIR/ensigns"

sudo cp "$BIN_PATH" "$BIN_DIR/$BIN_NAME"
sudo chmod 755 "$BIN_DIR/$BIN_NAME"
ok "Binary installed to $BIN_DIR/$BIN_NAME"

# Copy room and ensign configs if present
cp -n rooms/*.json "$INSTALL_DIR/rooms/" 2>/dev/null || true
cp -n ensigns/*.json "$INSTALL_DIR/ensigns/" 2>/dev/null || true

# Copy service file from deploy/ or scripts/
SRC_SERVICE=""
if [[ -f "$REPO_ROOT/deploy/hermes-construct.service" ]]; then
  SRC_SERVICE="$REPO_ROOT/deploy/hermes-construct.service"
elif [[ -f "$REPO_ROOT/scripts/hermes-construct.service" ]]; then
  SRC_SERVICE="$REPO_ROOT/scripts/hermes-construct.service"
fi

if [[ -n "$SRC_SERVICE" ]]; then
  sudo cp "$SRC_SERVICE" "$SERVICE_FILE"
  ok "Systemd unit installed"
else
  warn "No service file found — you'll need to create $SERVICE_FILE manually"
fi

# ── Step 7: Configure environment ───────────────────────────────────────────
sudo mkdir -p "$(dirname "$ENV_FILE")"

if [[ ! -f "$ENV_FILE" ]]; then
  info "Creating environment template at $ENV_FILE"
  sudo tee "$ENV_FILE" > /dev/null <<'ENVEOF'
# hermes-construct environment — fill in your tokens
TELEGRAM_BOT_TOKEN=
DEEPINFRA_API_KEY=
ZAI_API_KEY=
RUST_LOG=info
ENVEOF
  sudo chmod 600 "$ENV_FILE"
  sudo chown root:root "$ENV_FILE"
  warn "Edit $ENV_FILE to add your API tokens before starting!"
else
  ok "Environment file already exists at $ENV_FILE"
fi

# State directory
sudo mkdir -p "$STATE_DIR"
sudo chown hermes:hermes "$STATE_DIR"
sudo chmod 750 "$STATE_DIR"

# Set ownership
sudo chown -R hermes:hermes "$INSTALL_DIR"

# ── Step 8: Enable and start service ────────────────────────────────────────
info "Enabling systemd service..."
sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"

if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
  info "Service is running — restarting to pick up new binary..."
  sudo systemctl restart "$SERVICE_NAME"
else
  info "Starting service..."
  sudo systemctl start "$SERVICE_NAME"
fi

# ── Step 9: Health check ────────────────────────────────────────────────────
info "Waiting for service to start..."
sleep 3

if systemctl is-active --quiet "$SERVICE_NAME"; then
  ok "Service ${SERVICE_NAME} is active (running)"
else
  warn "Service not active yet. Check logs:"
  echo "  sudo journalctl -u $SERVICE_NAME -n 50 --no-pager"
fi

# Try HTTP health check
HEALTH_URL="http://localhost:8080/health"
if command -v curl &>/dev/null; then
  if curl -sf --max-time 5 "$HEALTH_URL" &>/dev/null; then
    ok "Health check passed: $HEALTH_URL"
  else
    info "Health endpoint at $HEALTH_URL not reachable (may not be configured yet)"
  fi
fi

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
ok "═══════════════════════════════════════════════════════"
ok "  hermes-construct deployed successfully!"
ok "═══════════════════════════════════════════════════════"
echo ""
echo "  Binary:      $BIN_DIR/$BIN_NAME"
echo "  Config:      $ENV_FILE"
echo "  State:       $STATE_DIR"
echo "  Service:     systemctl status $SERVICE_NAME"
echo "  Logs:        journalctl -u $SERVICE_NAME -f"
echo ""
if grep -q 'TELEGRAM_BOT_TOKEN=$' "$ENV_FILE" 2>/dev/null; then
  warn "Action needed: edit $ENV_FILE and add your tokens"
  echo "  sudo nano $ENV_FILE"
  echo "  sudo systemctl restart $SERVICE_NAME"
fi
