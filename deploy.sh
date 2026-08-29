#!/usr/bin/env bash
set -e

REMOTE_HOST="192.168.0.108"
REMOTE_USER="vmko"
REMOTE_PASS="vmko"
TARGET_BIN="frutiger"

echo "==> Building Frutiger DE (release, limited CPU/IO priority)..."
nice -n 15 ionice -c 3 cargo build -j 4 --release --bin "${TARGET_BIN}"

echo "==> Uploading binary and desktop session to ${REMOTE_HOST}..."
sshpass -p "${REMOTE_PASS}" scp -o StrictHostKeyChecking=no target/release/${TARGET_BIN} ${REMOTE_USER}@${REMOTE_HOST}:/tmp/${TARGET_BIN}
sshpass -p "${REMOTE_PASS}" scp -o StrictHostKeyChecking=no resources/frutiger.desktop ${REMOTE_USER}@${REMOTE_HOST}:/tmp/frutiger.desktop
sshpass -p "${REMOTE_PASS}" scp -o StrictHostKeyChecking=no resources/frutiger-session ${REMOTE_USER}@${REMOTE_HOST}:/tmp/frutiger-session
sshpass -p "${REMOTE_PASS}" scp -o StrictHostKeyChecking=no resources/frutiger.service ${REMOTE_USER}@${REMOTE_HOST}:/tmp/frutiger.service

echo "==> Installing Frutiger session on laptop..."
sshpass -p "${REMOTE_PASS}" ssh -o StrictHostKeyChecking=no ${REMOTE_USER}@${REMOTE_HOST} "
    echo '${REMOTE_PASS}' | sudo -S cp --remove-destination /tmp/${TARGET_BIN} /usr/bin/${TARGET_BIN}
    echo '${REMOTE_PASS}' | sudo -S cp --remove-destination /tmp/frutiger-session /usr/bin/frutiger-session
    echo '${REMOTE_PASS}' | sudo -S chmod +x /usr/bin/${TARGET_BIN} /usr/bin/frutiger-session
    echo '${REMOTE_PASS}' | sudo -S cp /tmp/frutiger.desktop /usr/share/wayland-sessions/frutiger.desktop
    echo '${REMOTE_PASS}' | sudo -S cp /tmp/frutiger.service /usr/lib/systemd/user/frutiger.service
    echo '${REMOTE_PASS}' | sudo -S ln -sf /usr/bin/${TARGET_BIN} /usr/bin/niri 2>/dev/null || true
    systemctl --user stop frutiger.service niri.service 2>/dev/null || true
    systemctl --user daemon-reload 2>/dev/null || true
    echo '${REMOTE_PASS}' | sudo -S systemctl restart sddm
"
echo "==> Frutiger DE successfully deployed to laptop!"
