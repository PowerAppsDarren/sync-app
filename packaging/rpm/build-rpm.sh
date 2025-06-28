#!/bin/bash
set -euo pipefail

# RPM package build script for sync-app

VERSION=${1:-$(grep '^version' ../../Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}
ARCHITECTURE=${2:-"x86_64"}
PACKAGE_NAME="sync-app"

echo "Building RPM package for ${PACKAGE_NAME} v${VERSION} (${ARCHITECTURE})"

# Map architecture names
RUST_TARGET=""
case "$ARCHITECTURE" in
    "x86_64")
        RUST_TARGET="x86_64-unknown-linux-gnu"
        ;;
    "aarch64")
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

# Set up RPM build environment
RPM_BUILD_ROOT="$HOME/rpmbuild"
mkdir -p "$RPM_BUILD_ROOT"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

# Create source tarball
TARBALL_DIR="/tmp/${PACKAGE_NAME}-${VERSION}"
rm -rf "$TARBALL_DIR"
mkdir -p "$TARBALL_DIR"

# Copy binaries to tarball
cp "$BINARY_DIR/sync" "$TARBALL_DIR/"
cp "$BINARY_DIR/sync-server" "$TARBALL_DIR/"
cp "$BINARY_DIR/sync-daemon" "$TARBALL_DIR/"
cp "$BINARY_DIR/pocketbase" "$TARBALL_DIR/"

# Create basic README if it doesn't exist
if [ ! -f "../../README.md" ]; then
    cat > "$TARBALL_DIR/README.md" << EOF
# Sync App

Cross-platform synchronization application with PocketBase backend.

## Components

- \`sync\`: Command-line interface
- \`sync-server\`: Server component
- \`sync-daemon\`: Background daemon service
- \`pocketbase\`: PocketBase database (optional)

For more information, visit: https://github.com/yourusername/sync-app
EOF
else
    cp "../../README.md" "$TARBALL_DIR/"
fi

# Create tarball
cd /tmp
tar czf "$RPM_BUILD_ROOT/SOURCES/${PACKAGE_NAME}-${VERSION}.tar.gz" "${PACKAGE_NAME}-${VERSION}/"
rm -rf "$TARBALL_DIR"

# Copy spec file with date substitution
SPEC_FILE="$RPM_BUILD_ROOT/SPECS/${PACKAGE_NAME}.spec"
cp sync-app.spec "$SPEC_FILE"

# Replace %{date} with actual date
DATE_STR=$(date '+%a %b %d %Y')
sed -i "s/%{date}/$DATE_STR/g" "$SPEC_FILE"

# Build the package
echo "Building RPM package..."
cd "$RPM_BUILD_ROOT"

# Build source RPM first
rpmbuild -bs "SPECS/${PACKAGE_NAME}.spec"

# Build binary RPM
rpmbuild -bb "SPECS/${PACKAGE_NAME}.spec" --target "$ARCHITECTURE"

# Find the created RPM
RPM_FILE=$(find RPMS -name "${PACKAGE_NAME}-${VERSION}-*.${ARCHITECTURE}.rpm" | head -n 1)
SRPM_FILE=$(find SRPMS -name "${PACKAGE_NAME}-${VERSION}-*.src.rpm" | head -n 1)

if [ -n "$RPM_FILE" ] && [ -f "$RPM_FILE" ]; then
    # Copy to our directory
    cp "$RPM_FILE" "$(dirname "$0")/"
    echo "RPM package created: $(basename "$RPM_FILE")"
    
    if [ -n "$SRPM_FILE" ] && [ -f "$SRPM_FILE" ]; then
        cp "$SRPM_FILE" "$(dirname "$0")/"
        echo "Source RPM created: $(basename "$SRPM_FILE")"
    fi
    
    echo ""
    echo "To install:"
    echo "  sudo rpm -ivh $(basename "$RPM_FILE")"
    echo "  # or on newer systems:"
    echo "  sudo dnf install $(basename "$RPM_FILE")"
    echo ""
    echo "To remove:"
    echo "  sudo rpm -e $PACKAGE_NAME"
    echo "  # or:"
    echo "  sudo dnf remove $PACKAGE_NAME"
    
else
    echo "Error: RPM package build failed"
    exit 1
fi
