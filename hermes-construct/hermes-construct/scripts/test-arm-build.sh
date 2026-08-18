#!/usr/bin/env bash
#
# test-arm-build.sh — Dry-run validation of the ARM build pipeline.
# Checks toolchain, cross-compilation config, service files, and attempts
# a build without deploying anything.
#
# Usage: ./test-arm-build.sh [--check-only] [--help]
#
set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; NC='\033[0m'

PASS=0; FAIL=0

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
pass()  { echo -e "${GREEN}[PASS]${NC}  $*"; PASS=$((PASS+1)); }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; FAIL=$((FAIL+1)); }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
die()   { echo -e "${RED}[FATAL]${NC} $*" >&2; exit 1; }

# ── Help ────────────────────────────────────────────────────────────────────
usage() {
cat <<'EOF'
Usage: test-arm-build.sh [OPTIONS]

Dry-run ARM build pipeline validation.

Options:
  --check-only    Only run checks, skip cargo build
  --help          Show this help message
EOF
exit 0
}

CHECK_ONLY=false
for arg in "$@"; do
  case "$arg" in
    --check-only) CHECK_ONLY=true ;;
    --help|-h)    usage ;;
    *) die "Unknown argument: $arg" ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

TARGET="aarch64-unknown-linux-gnu"
BIN_NAME="hermes-construct"

info "═══════════════════════════════════════════════════════"
info "  hermes-construct ARM build pipeline — dry run"
info "═══════════════════════════════════════════════════════"
echo ""

# ── Check 1: Rust toolchain ─────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
  pass "Cargo found: $(cargo --version)"
else
  fail "Cargo not found — install Rust: https://rustup.rs"
fi

if command -v rustup &>/dev/null; then
  pass "Rustup found"
else
  warn "Rustup not found (may be system Rust)"
fi

# ── Check 2: Cross-compilation target ───────────────────────────────────────
if command -v rustup &>/dev/null; then
  if rustup target list --installed | grep -q "$TARGET"; then
    pass "Target $TARGET is installed"
  else
    fail "Target $TARGET NOT installed. Run: rustup target add $TARGET"
    info "  Note: if building natively on ARM, this check can be ignored"
  fi
fi

# ── Check 3: .cargo/config.toml ─────────────────────────────────────────────
CARGO_CONFIG=".cargo/config.toml"
if [[ -f "$CARGO_CONFIG" ]]; then
  if grep -q "aarch64-unknown-linux-gnu" "$CARGO_CONFIG"; then
    LINKER="$(grep 'linker' "$CARGO_CONFIG" | head -1 | sed 's/.*=.*"\(.*\)"/\1/')"
    pass ".cargo/config.toml has aarch64 target (linker: $LINKER)"
  else
    fail ".cargo/config.toml missing aarch64-unknown-linux-gnu target"
  fi
else
  fail ".cargo/config.toml not found"
fi

# ── Check 4: Cross-linker availability ──────────────────────────────────────
if command -v aarch64-linux-gnu-gcc &>/dev/null; then
  pass "Cross-linker aarch64-linux-gnu-gcc found"
else
  warn "Cross-linker aarch64-linux-gnu-gcc not found (not needed for native ARM builds)"
  info "  Install on Debian/Ubuntu: sudo apt install gcc-aarch64-linux-gnu"
fi

# ── Check 5: Systemd service file ──────────────────────────────────────────
SERVICE_FOUND=false
for f in deploy/hermes-construct.service scripts/hermes-construct.service; do
  if [[ -f "$f" ]]; then
    pass "Service file found: $f"
    SERVICE_FOUND=true
    # Validate key fields
    grep -q "User=hermes" "$f" && pass "  → runs as 'hermes' user" || fail "  → missing User=hermes"
    grep -q "WorkingDirectory=/opt/hermes-construct" "$f" && pass "  → WorkingDirectory set" || fail "  → missing WorkingDirectory"
    grep -q "RestartSec=5" "$f" && pass "  → RestartSec=5" || warn "  → RestartSec not set to 5"
    grep -q "Restart=on-failure" "$f" && pass "  → Restart=on-failure" || fail "  → missing Restart=on-failure"
    break
  fi
done
[[ "$SERVICE_FOUND" == true ]] || fail "No systemd service file found"

# ── Check 6: Deploy scripts ─────────────────────────────────────────────────
for script in scripts/deploy-oracle.sh scripts/deploy-jetson.sh; do
  if [[ -f "$script" ]]; then
    if [[ -x "$script" ]]; then
      pass "$script is executable"
    else
      fail "$script exists but is not executable"
    fi
    head -1 "$script" | grep -q bash && pass "  → has bash shebang" || fail "  → missing bash shebang"
    grep -q "set -euo pipefail" "$script" && pass "  → has error handling (set -euo pipefail)" || fail "  → missing set -euo pipefail"
    grep -q "\-\-help" "$script" && pass "  → has --help flag" || fail "  → missing --help flag"
  else
    fail "$script not found"
  fi
done

# ── Check 7: Jetson env ─────────────────────────────────────────────────────
if [[ -f "deploy/jetson-env.sh" ]]; then
  pass "deploy/jetson-env.sh found"
  grep -q "RAYON_NUM_THREADS" "$f" 2>/dev/null && pass "  → RAYON_NUM_THREADS set" || true
  grep -q "TOKIO_WORKER_THREADS" "$f" 2>/dev/null && pass "  → TOKIO_WORKER_THREADS set" || true
  grep -q "HERMES_MAX_ROOMS" "$f" 2>/dev/null && pass "  → HERMES_MAX_ROOMS set" || true
else
  fail "deploy/jetson-env.sh not found"
fi

# ── Check 8: Cargo.toml sanity ──────────────────────────────────────────────
if [[ -f "Cargo.toml" ]]; then
  pass "Cargo.toml exists"
  grep -q 'name = "hermes-construct"' Cargo.toml && pass "  → package name correct" || fail "  → package name mismatch"
  grep -q "tokio" Cargo.toml && pass "  → tokio dependency present" || fail "  → missing tokio"
  grep -q "rusqlite" Cargo.toml && pass "  → rusqlite dependency present" || fail "  → missing rusqlite"
else
  fail "Cargo.toml not found"
fi

# ── Check 9: Source files ───────────────────────────────────────────────────
if [[ -f "src/main.rs" ]]; then
  pass "src/main.rs exists"
else
  fail "src/main.rs not found"
fi

# ── Check 10: Attempt build (unless --check-only) ───────────────────────────
if [[ "$CHECK_ONLY" == false ]]; then
  echo ""
  info "Attempting ARM build..."

  ARCH="$(uname -m)"
  if [[ "$ARCH" == "aarch64" ]]; then
    info "Native ARM build..."
    if cargo build --release 2>&1; then
      pass "Native ARM build succeeded"
      BIN_PATH="target/release/$BIN_NAME"
      if [[ -f "$BIN_PATH" ]]; then
        pass "Binary created: $(du -h "$BIN_PATH" | cut -f1)"
      fi
    else
      fail "Native ARM build failed"
    fi
  else
    info "Cross-compiling for $TARGET..."
    if cargo build --release --target "$TARGET" 2>&1; then
      pass "Cross-compilation build succeeded"
      BIN_PATH="target/$TARGET/release/$BIN_NAME"
      if [[ -f "$BIN_PATH" ]]; then
        pass "Binary created: $(du -h "$BIN_PATH" | cut -f1)"
        file "$BIN_PATH" | grep -q "aarch64" && pass "  → binary is aarch64" || fail "  → binary architecture mismatch"
      fi
    else
      fail "Cross-compilation build failed (expected if cross-linker not installed)"
    fi
  fi
fi

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
info "═══════════════════════════════════════════════════════"
if [[ "$FAIL" -eq 0 ]]; then
  pass "All $PASS checks passed!"
else
  echo -e "  ${GREEN}Passed: $PASS${NC}  ${RED}Failed: $FAIL${NC}"
fi
info "═══════════════════════════════════════════════════════"

[[ "$FAIL" -eq 0 ]]
