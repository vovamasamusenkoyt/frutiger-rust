#!/usr/bin/env bash
set -e

REMOTE_HOST="192.168.0.108"
REMOTE_USER="vmko"
REMOTE_PASS="vmko"
TARGET_BIN="frutiger-de"

echo "==> Building Frutiger Rust (release, limited CPU/IO priority)..."
nice -n 15 ionice -c 3 cargo build -j 4 --release --bin "${TARGET_BIN}"

echo "==> Uploading binary to ${REMOTE_HOST}..."
sshpass -p "${REMOTE_PASS}" scp -o StrictHostKeyChecking=no target/release/${TARGET_BIN} ${REMOTE_USER}@${REMOTE_HOST}:/tmp/${TARGET_BIN}

echo "==> Setting executable permissions and launching on remote laptop..."
sshpass -p "${REMOTE_PASS}" ssh -o StrictHostKeyChecking=no ${REMOTE_USER}@${REMOTE_HOST} "
    echo '${REMOTE_PASS}' | sudo -S systemctl stop sddm 2>/dev/null || true
    echo '${REMOTE_PASS}' | sudo -S chmod +x /tmp/${TARGET_BIN}
    echo '=== Starting Frutiger Rust DE (DRM/KMS Backend) ==='
    echo '${REMOTE_PASS}' | sudo -S WAYLAND_DISPLAY=frutiger-0 RUST_LOG=info /tmp/${TARGET_BIN} --backend drm
"
