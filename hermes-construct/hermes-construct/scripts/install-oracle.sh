#!/usr/bin/env bash
#
# install-oracle.sh — install hermes-construct on the Oracle ARM box.
# Run from inside the extracted deployment bundle, as root:
#
#   sudo ./install-oracle.sh
#
# Creates the hermes system user, installs the binary + configs under
# /opt/hermes-construct, sets up the secrets env file under /etc, and
# installs + enables the systemd service.
#
set -euo pipefail

HERMES_USER="hermes"
BIN_NAME="hermes-construct"
INSTALL_DIR="/opt/hermes-construct"
ENV_DIR="/etc/hermes-construct"
ENV_FILE="$ENV_DIR/hermes.env"
UNIT_DEST="/etc/systemd/system/hermes-construct.service"

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$(id -u)" -ne 0 ]; then
  echo "ERROR: must run as root (try: sudo $0)" >&2
  exit 1
fi

# Sanity check: confirm the bundle contents are present.
for f in "bin/$BIN_NAME" "hermes-construct.service" "hermes.env.example"; do
  if [ ! -e "$SRC_DIR/$f" ]; then
    echo "ERROR: bundle file missing: $f (run from the extracted tarball)" >&2
    exit 1
  fi
done

# Refuse to install an x86 binary on the ARM box by mistake.
if command -v file >/dev/null 2>&1; then
  if ! file "$SRC_DIR/bin/$BIN_NAME" | grep -q "aarch64"; then
    echo "ERROR: $BIN_NAME is not an aarch64 binary — wrong build?" >&2
    file "$SRC_DIR/bin/$BIN_NAME" >&2
    exit 1
  fi
fi

echo "==> Creating system user '$HERMES_USER'"
if id "$HERMES_USER" >/dev/null 2>&1; then
  echo "    user already exists"
else
  useradd --system --home-dir "$INSTALL_DIR" --shell /usr/sbin/nologin "$HERMES_USER"
fi

echo "==> Installing binary and configs to $INSTALL_DIR"
install -d -o "$HERMES_USER" -g "$HERMES_USER" -m 0755 \
  "$INSTALL_DIR" "$INSTALL_DIR/bin" "$INSTALL_DIR/rooms" "$INSTALL_DIR/ensigns"

install -o "$HERMES_USER" -g "$HERMES_USER" -m 0755 \
  "$SRC_DIR/bin/$BIN_NAME" "$INSTALL_DIR/bin/$BIN_NAME"

if compgen -G "$SRC_DIR/rooms/*.json" >/dev/null; then
  install -o "$HERMES_USER" -g "$HERMES_USER" -m 0644 "$SRC_DIR"/rooms/*.json "$INSTALL_DIR/rooms/"
fi
if compgen -G "$SRC_DIR/ensigns/*.json" >/dev/null; then
  install -o "$HERMES_USER" -g "$HERMES_USER" -m 0644 "$SRC_DIR"/ensigns/*.json "$INSTALL_DIR/ensigns/"
fi

echo "==> Setting up secrets file $ENV_FILE"
install -d -m 0750 "$ENV_DIR"
if [ -f "$ENV_FILE" ]; then
  echo "    $ENV_FILE already exists — leaving it untouched"
else
  install -m 0600 "$SRC_DIR/hermes.env.example" "$ENV_FILE"
  echo "    created $ENV_FILE — EDIT IT to add your tokens before starting"
fi

echo "==> Installing systemd unit"
install -m 0644 "$SRC_DIR/hermes-construct.service" "$UNIT_DEST"
systemctl daemon-reload
systemctl enable hermes-construct.service

cat <<EOF

================================================================
 hermes-construct installed.
================================================================
 Binary:   $INSTALL_DIR/bin/$BIN_NAME
 Configs:  $INSTALL_DIR/rooms , $INSTALL_DIR/ensigns
 Secrets:  $ENV_FILE  (0600, root-only)
 State/DB: /var/lib/hermes-construct/universe.db (auto-created)

 Next steps:
   1. sudo nano $ENV_FILE        # add TELEGRAM_BOT_TOKEN + provider key(s)
   2. sudo systemctl start hermes-construct
   3. journalctl -u hermes-construct -f

 Stop gracefully with: sudo systemctl stop hermes-construct
 (sends SIGINT -> stand down ensigns, save state, checkpoint WAL)
================================================================
EOF
