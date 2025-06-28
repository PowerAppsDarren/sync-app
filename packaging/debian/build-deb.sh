#!/bin/bash
set -euo pipefail

# Debian package build script for sync-app

VERSION=${1:-$(grep '^version' ../../Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}
ARCHITECTURE=${2:-"amd64"}
BUILD_DIR="build"
PACKAGE_NAME="sync-app"

echo "Building Debian package for ${PACKAGE_NAME} v${VERSION} (${ARCHITECTURE})"

# Clean and create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Create package directory structure
PACKAGE_DIR="$BUILD_DIR/${PACKAGE_NAME}_${VERSION}_${ARCHITECTURE}"
mkdir -p "$PACKAGE_DIR"/{DEBIAN,usr/bin,usr/share/doc/$PACKAGE_NAME,etc/$PACKAGE_NAME,lib/systemd/system,usr/share/man/man1}

# Map architecture names
RUST_TARGET=""
case "$ARCHITECTURE" in
    "amd64")
        RUST_TARGET="x86_64-unknown-linux-gnu"
        ;;
    "arm64")
        RUST_TARGET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Unsupported architecture: $ARCHITECTURE"
        exit 1
        ;;
esac

# Check if binaries exist
BINARY_DIR="../../target/release-builds/$RUST_TARGET"
if [ ! -d "$BINARY_DIR" ]; then
    echo "Error: Binaries not found in $BINARY_DIR"
    echo "Please run the build script first: ../../scripts/build-release.sh"
    exit 1
fi

# Copy binaries
cp "$BINARY_DIR/sync" "$PACKAGE_DIR/usr/bin/"
cp "$BINARY_DIR/sync-server" "$PACKAGE_DIR/usr/bin/"
cp "$BINARY_DIR/sync-daemon" "$PACKAGE_DIR/usr/bin/"
cp "$BINARY_DIR/pocketbase" "$PACKAGE_DIR/usr/bin/"

# Create control file
cat > "$PACKAGE_DIR/DEBIAN/control" << EOF
Package: $PACKAGE_NAME
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCHITECTURE
Depends: libc6, libssl3 | libssl1.1
Recommends: sqlite3
Suggests: nginx, apache2
Maintainer: Your Name <your.email@example.com>
Description: Cross-platform synchronization application with PocketBase backend
 Sync App is a modern, cross-platform synchronization application built with
 Rust and powered by PocketBase. It provides multiple interfaces including
 a command-line tool, server component, and background daemon.
 .
 Features:
  - Cross-platform support (Linux, macOS, Windows)
  - Real-time synchronization across devices
  - PocketBase integration for backend database and API
  - Secure communication with built-in encryption
  - High performance with Rust implementation
Homepage: https://github.com/yourusername/sync-app
EOF

# Create postinst script
cat > "$PACKAGE_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/bash
set -e

# Create sync-app user and group
if ! getent group sync-app >/dev/null; then
    addgroup --system sync-app
fi

if ! getent passwd sync-app >/dev/null; then
    adduser --system --home /var/lib/sync-app --shell /bin/false \
            --gecos "Sync App daemon" --ingroup sync-app sync-app
fi

# Create directories
mkdir -p /var/lib/sync-app
mkdir -p /var/log/sync-app
chown sync-app:sync-app /var/lib/sync-app
chown sync-app:sync-app /var/log/sync-app

# Set permissions
chmod 755 /usr/bin/sync
chmod 755 /usr/bin/sync-server  
chmod 755 /usr/bin/sync-daemon
chmod 755 /usr/bin/pocketbase

# Enable and start systemd service
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload
    systemctl enable sync-daemon.service
    echo "Sync daemon service enabled. Start with: systemctl start sync-daemon"
fi

echo "Sync App installed successfully!"
echo "Configuration: /etc/sync-app/config.yaml"
echo "Commands: sync, sync-server, sync-daemon, pocketbase"
EOF

# Create prerm script
cat > "$PACKAGE_DIR/DEBIAN/prerm" << 'EOF'
#!/bin/bash
set -e

# Stop and disable service
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop sync-daemon.service 2>/dev/null || true
    systemctl disable sync-daemon.service 2>/dev/null || true
fi
EOF

# Create postrm script  
cat > "$PACKAGE_DIR/DEBIAN/postrm" << 'EOF'
#!/bin/bash
set -e

if [ "$1" = "purge" ]; then
    # Remove user and group
    if getent passwd sync-app >/dev/null; then
        deluser sync-app 2>/dev/null || true
    fi
    
    if getent group sync-app >/dev/null; then
        delgroup sync-app 2>/dev/null || true
    fi
    
    # Remove data directories
    rm -rf /var/lib/sync-app
    rm -rf /var/log/sync-app
    rm -rf /etc/sync-app
fi

# Reload systemd
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload
fi
EOF

# Make scripts executable
chmod 755 "$PACKAGE_DIR/DEBIAN/postinst"
chmod 755 "$PACKAGE_DIR/DEBIAN/prerm"
chmod 755 "$PACKAGE_DIR/DEBIAN/postrm"

# Create default configuration
cat > "$PACKAGE_DIR/etc/$PACKAGE_NAME/config.yaml" << EOF
server:
  host: "127.0.0.1"
  port: 8080
  
database:
  path: "/var/lib/sync-app/sync.db"
  
logging:
  level: "info"
  file: "/var/log/sync-app/sync.log"
  
sync:
  interval: "30s"
  auto_start: false
EOF

# Create systemd service file
cat > "$PACKAGE_DIR/lib/systemd/system/sync-daemon.service" << EOF
[Unit]
Description=Sync App Daemon
Documentation=https://github.com/yourusername/sync-app
After=network.target
Wants=network.target

[Service]
Type=simple
User=sync-app
Group=sync-app
WorkingDirectory=/var/lib/sync-app
ExecStart=/usr/bin/sync-daemon --config /etc/sync-app/config.yaml
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=5
TimeoutStopSec=20
KillMode=mixed

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/sync-app /var/log/sync-app
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
EOF

# Create documentation
cat > "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/README.Debian" << EOF
Sync App for Debian
===================

This package provides the Sync App synchronization suite for Debian systems.

Components:
-----------
- sync: Command-line interface
- sync-server: Server component
- sync-daemon: Background daemon service
- pocketbase: PocketBase database (optional)

Configuration:
--------------
System configuration: /etc/sync-app/config.yaml
User data: /var/lib/sync-app/
Logs: /var/log/sync-app/

Service Management:
-------------------
The sync-daemon is installed as a systemd service:

  sudo systemctl start sync-daemon    # Start the service
  sudo systemctl stop sync-daemon     # Stop the service
  sudo systemctl enable sync-daemon   # Enable auto-start
  sudo systemctl disable sync-daemon  # Disable auto-start
  sudo systemctl status sync-daemon   # Check status

Usage:
------
See the man pages: man sync, man sync-server, man sync-daemon

For more information, visit:
https://github.com/yourusername/sync-app
EOF

# Create copyright file
cat > "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/copyright" << EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: sync-app
Upstream-Contact: Your Name <your.email@example.com>
Source: https://github.com/yourusername/sync-app

Files: *
Copyright: 2024 Your Name <your.email@example.com>
License: AGPL-3.0

License: AGPL-3.0
 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU Affero General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.
 .
 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU Affero General Public License for more details.
 .
 You should have received a copy of the GNU Affero General Public License
 along with this program.  If not, see <https://www.gnu.org/licenses/>.
 .
 On Debian systems, the complete text of the GNU Affero General
 Public License version 3 can be found in "/usr/share/common-licenses/AGPL-3".
EOF

# Create changelog
cat > "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/changelog.Debian.gz" << EOF
sync-app ($VERSION) unstable; urgency=low

  * Initial Debian package release
  * Cross-platform synchronization with PocketBase backend
  * CLI tool, server component, and daemon service
  * Systemd service integration

 -- Your Name <your.email@example.com>  $(date -R)
EOF
gzip "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/changelog.Debian.gz"

# Calculate installed size
INSTALLED_SIZE=$(du -sk "$PACKAGE_DIR" | cut -f1)
echo "Installed-Size: $INSTALLED_SIZE" >> "$PACKAGE_DIR/DEBIAN/control"

# Build the package
echo "Building package..."
dpkg-deb --build "$PACKAGE_DIR"

# Move the package to the current directory
mv "$BUILD_DIR/${PACKAGE_NAME}_${VERSION}_${ARCHITECTURE}.deb" .

echo "Debian package created: ${PACKAGE_NAME}_${VERSION}_${ARCHITECTURE}.deb"
echo ""
echo "To install:"
echo "  sudo dpkg -i ${PACKAGE_NAME}_${VERSION}_${ARCHITECTURE}.deb"
echo "  sudo apt-get install -f  # If there are dependency issues"
echo ""
echo "To remove:"
echo "  sudo apt-get remove $PACKAGE_NAME"
echo "  sudo apt-get purge $PACKAGE_NAME  # Remove all data"
