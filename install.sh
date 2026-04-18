#!/bin/bash
set -e

REPO="juanm512/evlogstudio"
SERVICE_NAME="evlogstudio"

# Recomendado: curl ... -o /tmp/install.sh && sudo bash /tmp/install.sh
# Sin root:    curl ... -o /tmp/install.sh && bash /tmp/install.sh

if [ "$(id -u)" = "0" ]; then
  INSTALL_DIR="/usr/local/bin"
  DATA_DIR="/var/lib/evlogstudio"
else
  INSTALL_DIR="$HOME/.local/bin"
  DATA_DIR="$HOME/.local/share/evlogstudio"
  mkdir -p "$INSTALL_DIR"
fi

# ── detección de OS y arquitectura ──────────────────────────────

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64)  ARCH="amd64" ;;
  aarch64) ARCH="arm64" ;;
  arm64)   ARCH="arm64" ;;
  *)
    echo "Architecture not supported: $ARCH"
    exit 1
    ;;
esac

case "$OS" in
  linux)  ;;
  darwin) ;;
  *)
    echo "OS not supported: $OS"
    exit 1
    ;;
esac

ARTIFACT="evlogstudio-${OS}-${ARCH}"
LATEST_URL="https://github.com/${REPO}/releases/latest/download/${ARTIFACT}"

echo "Detected: ${OS}/${ARCH}"
echo "Downloading evlogstudio..."

# ── descarga ─────────────────────────────────────────────────────

curl -sSL "$LATEST_URL" -o "/tmp/evlogstudio"
chmod +x "/tmp/evlogstudio"

# ── verificar que funciona ────────────────────────────────────────

if ! /tmp/evlogstudio --version 2>/dev/null; then
  echo "The downloaded binary is not responding. Check the release."
  exit 1
fi

# ── instalar ──────────────────────────────────────────────────────

mv /tmp/evlogstudio "$INSTALL_DIR/evlogstudio"
echo "Binary installed in $INSTALL_DIR/evlogstudio"

# ── preguntar configuración ───────────────────────────────────────

echo ""
echo "Configuration:"
read -p "  Puerto [8080]: " PORT
PORT=${PORT:-8080}

read -p "  Storage mode (local/motherduck/s3) [local]: " STORAGE_MODE
STORAGE_MODE=${STORAGE_MODE:-local}

DATA_PATH_DEFAULT="$DATA_DIR/logs.duckdb"
if [ "$STORAGE_MODE" = "local" ]; then
  read -p "  DuckDB file path [$DATA_PATH_DEFAULT]: " DATA_PATH
  DATA_PATH=${DATA_PATH:-$DATA_PATH_DEFAULT}
  mkdir -p "$(dirname "$DATA_PATH")"
fi

# ── instalar systemd service (solo Linux) ────────────────────────

if [ "$OS" = "linux" ] && command -v systemctl >/dev/null 2>&1; then
  read -p "Install as systemd service? [Y/n]: " INSTALL_SERVICE
  INSTALL_SERVICE=${INSTALL_SERVICE:-Y}

  case "$INSTALL_SERVICE" in
    [Yy]*)
      SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

      ENV_LINES="Environment=PORT=${PORT}
Environment=STORAGE_MODE=${STORAGE_MODE}"
      if [ "$STORAGE_MODE" = "local" ]; then
        ENV_LINES="${ENV_LINES}
Environment=DATA_PATH=${DATA_PATH}"
      fi

      cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=evlogstudio log server
After=network.target

[Service]
Type=simple
User=root
${ENV_LINES}
ExecStart=$INSTALL_DIR/evlogstudio
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

      systemctl daemon-reload
      systemctl enable "$SERVICE_NAME"
      systemctl start "$SERVICE_NAME"

      echo ""
      echo "Service installed and started."
      echo "Useful commands:"
      echo "  systemctl status $SERVICE_NAME"
      echo "  systemctl restart $SERVICE_NAME"
      echo "  journalctl -u $SERVICE_NAME -f"
      ;;
  esac
fi

# ── mensaje final ─────────────────────────────────────────────────

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  evlogstudio installed correctly                 ║"
echo "║                                                  ║"
echo "║  Open the browser at:                            ║"
echo "║  http://YOUR-IP:${PORT}                          ║"
echo "║                                                  ║"
echo "║  Docs: github.com/${REPO}                        ║"
echo "╚══════════════════════════════════════════════════╝"
