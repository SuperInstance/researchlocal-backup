#!/usr/bin/env bash
#
# deploy-jetson.sh — Deploy hermes-construct on NVIDIA Jetson Nano/Orin.
# Detects model, configures for memory constraints, installs as systemd service.
#
# Usage: ./deploy-jetson.sh [--skip-build] [--help]
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
Usage: deploy-jetson.sh [OPTIONS]

Deploy hermes-construct on NVIDIA Jetson Nano/Orin with memory-optimized
configuration.

Options:
  --skip-build    Skip cargo build (use existing binary)
  --help          Show this help message

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
JETSON_ENV_FILE="/etc/hermes-construct/jetson-env.sh"
SERVICE_NAME="hermes-construct"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Step 1: Architecture & Jetson detection ─────────────────────────────────
ARCH="$(uname -m)"
info "Architecture: $ARCH"
[[ "$ARCH" == "aarch64" ]] || warn "Expected aarch64, got $ARCH"

JETSON_MODEL="unknown"
JETSON_MEM="unknown"

if [[ -f /etc/nv_tegra_release ]]; then
  TEGRA_VER="$(head -1 /etc/nv_tegra_release)"
  info "Tegra release: $TEGRA_VER"

  case "$TEGRA_VER" in
    *R32*)  JETSON_MODEL="Jetson Nano (JetPack 4.x)" ;;
    *R35*)  JETSON_MODEL="Jetson Orin NX (JetPack 5.x)" ;;
    *R36*)  JETSON_MODEL="Jetson Orin Nano (JetPack 6.x)" ;;
    *)      JETSON_MODEL="Unknown Jetson (${TEGRA_VER})" ;;
  esac
  ok "Detected: $JETSON_MODEL"
else
  warn "/etc/nv_tegra_release not found — not a Jetson device?"
  warn "Continuing anyway (may be a compatible ARM board)..."
fi

# Detect memory
TOTAL_MEM_KB="$(grep MemTotal /proc/meminfo | awk '{print $2}')"
TOTAL_MEM_MB="$((TOTAL_MEM_KB / 1024))"
info "System memory: ${TOTAL_MEM_MB}MB"

if [[ "$TOTAL_MEM_MB" -lt 2048 ]]; then
  warn "Low memory detected (${TOTAL_MEM_MB}MB) — applying strict limits"
  RAYON_THREADS=1
  TOKIO_THREADS=2
  MAX_ROOMS=2
elif [[ "$TOTAL_MEM_MB" -lt 4096 ]]; then
  info "Constrained memory (${TOTAL_MEM_MB}MB) — applying conservative limits"
  RAYON_THREADS=2
  TOKIO_THREADS=2
  MAX_ROOMS=3
else
  info "Adequate memory (${TOTAL_MEM_MB}MB) — standard Jetson profile"
  RAYON_THREADS=2
  TOKIO_THREADS=4
  MAX_ROOMS=5
fi

# ── Step 2: System dependencies ─────────────────────────────────────────────
info "Checking system dependencies..."

if ! command -v sqlite3 &>/dev/null; then
  info "Installing SQLite3..."
  sudo apt-get update -qq && sudo apt-get install -y -qq sqlite3 libsqlite3-dev
  ok "SQLite3 installed"
else
  ok "SQLite3 available"
fi

# ── Step 3: Rust toolchain ──────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
  ok "System Rust found: $(rustc --version)"
else
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
  ok "Rust installed: $(rustc --version)"
fi

# ── Step 4: Build with ARM optimizations ────────────────────────────────────
cd "$REPO_ROOT"

if [[ "$SKIP_BUILD" == true ]]; then
  info "Skipping build (--skip-build)"
else
  info "Building release binary with NEON optimizations (target-cpu=native)..."

  # Build natively on ARM with NEON optimizations
  RUSTFLAGS="-C target-cpu=native" cargo build --release

  BIN_PATH="target/release/$BIN_NAME"
  if [[ ! -f "$BIN_PATH" ]]; then
    die "Build failed — binary not found"
  fi
  ok "Build complete: $(du -h "$BIN_PATH" | cut -f1)"
fi

BIN_PATH="${REPO_ROOT}/target/release/${BIN_NAME}"
[[ -f "$BIN_PATH" ]] || die "Binary not found at $BIN_PATH"

# ── Step 5: Create user and install ─────────────────────────────────────────
if id "hermes" &>/dev/null; then
  ok "User 'hermes' exists"
else
  info "Creating system user 'hermes'..."
  sudo useradd --system --no-create-home --shell /usr/sbin/nologin hermes
  ok "User 'hermes' created"
fi

info "Installing to $INSTALL_DIR..."
sudo mkdir -p "$BIN_DIR" "$INSTALL_DIR/rooms" "$INSTALL_DIR/ensigns"
sudo cp "$BIN_PATH" "$BIN_DIR/$BIN_NAME"
sudo chmod 755 "$BIN_DIR/$BIN_NAME"

# Copy configs
cp -n rooms/*.json "$INSTALL_DIR/rooms/" 2>/dev/null || true
cp -n ensigns/*.json "$INSTALL_DIR/ensigns/" 2>/dev/null || true

# ── Step 6: Jetson environment overrides ─────────────────────────────────────
info "Writing Jetson environment profile to $JETSON_ENV_FILE"
sudo mkdir -p "$(dirname "$JETSON_ENV_FILE")"
sudo tee "$JETSON_ENV_FILE" > /dev/null <<JETENVEOF
# Jetson memory-constrained profile — auto-generated by deploy-jetson.sh
# Model: ${JETSON_MODEL}
# Memory: ${TOTAL_MEM_MB}MB
RAYON_NUM_THREADS=${RAYON_THREADS}
TOKIO_WORKER_THREADS=${TOKIO_THREADS}
HERMES_MAX_ROOMS=${MAX_ROOMS}
HERMES_CONSERVATION_BUDGET=50.0
JETENVEOF
sudo chmod 644 "$JETSON_ENV_FILE"
ok "Jetson env written: ${RAYON_THREADS} rayon, ${TOKIO_THREADS} tokio, ${MAX_ROOMS} rooms"

# ── Step 7: Main environment file ───────────────────────────────────────────
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
  warn "Edit $ENV_FILE to add your API tokens!"
else
  ok "Environment file exists"
fi

# ── Step 8: Systemd service ─────────────────────────────────────────────────
SRC_SERVICE=""
if [[ -f "$REPO_ROOT/deploy/hermes-construct.service" ]]; then
  SRC_SERVICE="$REPO_ROOT/deploy/hermes-construct.service"
elif [[ -f "$REPO_ROOT/scripts/hermes-construct.service" ]]; then
  SRC_SERVICE="$REPO_ROOT/scripts/hermes-construct.service"
fi

if [[ -n "$SRC_SERVICE" ]]; then
  sudo cp "$SRC_SERVICE" "$SERVICE_FILE"

  # Append Jetson env file if the service supports it
  if ! grep -q "jetson-env" "$SERVICE_FILE"; then
    # Add Jetson env as a supplementary EnvironmentFile
    sudo sed -i "/^EnvironmentFile=/a EnvironmentFile=${JETSON_ENV_FILE}" "$SERVICE_FILE"
  fi
  ok "Systemd unit installed with Jetson overrides"
fi

# State directory
sudo mkdir -p "$STATE_DIR"
sudo chown hermes:hermes "$STATE_DIR" "$INSTALL_DIR"
sudo chown -R hermes:hermes "$INSTALL_DIR"

# ── Step 9: Enable and start ────────────────────────────────────────────────
info "Enabling systemd service..."
sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"

if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
  sudo systemctl restart "$SERVICE_NAME"
else
  sudo systemctl start "$SERVICE_NAME"
fi

sleep 3

if systemctl is-active --quiet "$SERVICE_NAME"; then
  ok "Service ${SERVICE_NAME} is active"
else
  warn "Service not active yet. Check: journalctl -u $SERVICE_NAME -n 50"
fi

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
ok "═══════════════════════════════════════════════════════"
ok "  hermes-construct deployed on $JETSON_MODEL"
ok "═══════════════════════════════════════════════════════"
echo ""
echo "  Model:       $JETSON_MODEL"
echo "  Memory:      ${TOTAL_MEM_MB}MB"
echo "  Threads:     rayon=${RAYON_THREADS} tokio=${TOKIO_THREADS}"
echo "  Max rooms:   ${MAX_ROOMS}"
echo "  Binary:      $BIN_DIR/$BIN_NAME"
echo "  Jetson env:  $JETSON_ENV_FILE"
echo "  Secrets:     $ENV_FILE"
echo "  Logs:        journalctl -u $SERVICE_NAME -f"
