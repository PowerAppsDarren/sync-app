#!/bin/bash
set -euo pipefail

# Master packaging script for sync-app
# This script builds all package formats

VERSION=${1:-$(grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}
OUTPUT_DIR="target/packages"

echo "Building all packages for sync-app v${VERSION}"

# Clean and create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Ensure binaries are built first
echo "Building cross-platform binaries..."
if [ ! -d "target/release-builds" ]; then
    echo "Running build script first..."
    scripts/build-release.sh "$VERSION"
fi

# Build Debian packages
echo ""
echo "Building Debian packages..."
cd packaging/debian
chmod +x build-deb.sh

# Build for amd64
./build-deb.sh "$VERSION" "amd64"
mv sync-app_${VERSION}_amd64.deb "../../$OUTPUT_DIR/"

# Build for arm64  
./build-deb.sh "$VERSION" "arm64"
mv sync-app_${VERSION}_arm64.deb "../../$OUTPUT_DIR/"

cd ../..

# Build RPM packages (if rpmbuild is available)
echo ""
echo "Building RPM packages..."
if command -v rpmbuild >/dev/null 2>&1; then
    cd packaging/rpm
    chmod +x build-rpm.sh
    
    # Build for x86_64
    ./build-rpm.sh "$VERSION" "x86_64"
    mv sync-app-${VERSION}-*.x86_64.rpm "../../$OUTPUT_DIR/" 2>/dev/null || true
    mv sync-app-${VERSION}-*.src.rpm "../../$OUTPUT_DIR/" 2>/dev/null || true
    
    # Build for aarch64
    ./build-rpm.sh "$VERSION" "aarch64"  
    mv sync-app-${VERSION}-*.aarch64.rpm "../../$OUTPUT_DIR/" 2>/dev/null || true
    
    cd ../..
else
    echo "Warning: rpmbuild not found, skipping RPM packages"
fi

# Build Chocolatey package (if choco is available)
echo ""
echo "Building Chocolatey package..."
if command -v choco >/dev/null 2>&1; then
    cd packaging/chocolatey
    
    # Update version and checksum in nuspec
    WINDOWS_ARCHIVE="../../target/release-builds/sync-app-${VERSION}-x86_64-pc-windows-gnu.zip"
    if [ -f "$WINDOWS_ARCHIVE" ]; then
        CHECKSUM=$(sha256sum "$WINDOWS_ARCHIVE" | cut -d' ' -f1)
        
        # Update nuspec file
        sed -i "s/<version>.*<\/version>/<version>$VERSION<\/version>/" sync-app.nuspec
        sed -i "s/v0\.1\.0/v$VERSION/g" sync-app.nuspec
        
        # Update PowerShell script
        sed -i "s/SHA256_PLACEHOLDER/$CHECKSUM/" tools/chocolateyinstall.ps1
        sed -i "s/\$packageVersion/0.1.0/g" tools/chocolateyinstall.ps1
        
        # Build package
        choco pack sync-app.nuspec
        mv sync-app.*.nupkg "../../$OUTPUT_DIR/" 2>/dev/null || true
        
        # Restore files
        git checkout sync-app.nuspec tools/chocolateyinstall.ps1 2>/dev/null || true
    else
        echo "Warning: Windows binary not found, skipping Chocolatey package"
    fi
    
    cd ../..
else
    echo "Warning: choco not found, skipping Chocolatey package"
fi

# Update Homebrew formula
echo ""
echo "Updating Homebrew formula..."
cd packaging/homebrew

# Calculate checksums for macOS binaries
DARWIN_X64_ARCHIVE="../../target/release-builds/sync-app-${VERSION}-x86_64-apple-darwin.tar.gz"
DARWIN_ARM64_ARCHIVE="../../target/release-builds/sync-app-${VERSION}-aarch64-apple-darwin.tar.gz"
LINUX_X64_ARCHIVE="../../target/release-builds/sync-app-${VERSION}-x86_64-unknown-linux-musl.tar.gz"
LINUX_ARM64_ARCHIVE="../../target/release-builds/sync-app-${VERSION}-aarch64-unknown-linux-musl.tar.gz"

if [ -f "$DARWIN_X64_ARCHIVE" ]; then
    DARWIN_X64_SHA=$(sha256sum "$DARWIN_X64_ARCHIVE" | cut -d' ' -f1)
    sed -i "s/SHA256_X86_64_DARWIN_PLACEHOLDER/$DARWIN_X64_SHA/" sync-app.rb
fi

if [ -f "$DARWIN_ARM64_ARCHIVE" ]; then
    DARWIN_ARM64_SHA=$(sha256sum "$DARWIN_ARM64_ARCHIVE" | cut -d' ' -f1)  
    sed -i "s/SHA256_ARM64_DARWIN_PLACEHOLDER/$DARWIN_ARM64_SHA/" sync-app.rb
fi

if [ -f "$LINUX_X64_ARCHIVE" ]; then
    LINUX_X64_SHA=$(sha256sum "$LINUX_X64_ARCHIVE" | cut -d' ' -f1)
    sed -i "s/SHA256_X86_64_LINUX_PLACEHOLDER/$LINUX_X64_SHA/" sync-app.rb
fi

if [ -f "$LINUX_ARM64_ARCHIVE" ]; then
    LINUX_ARM64_SHA=$(sha256sum "$LINUX_ARM64_ARCHIVE" | cut -d' ' -f1)
    sed -i "s/SHA256_ARM64_LINUX_PLACEHOLDER/$LINUX_ARM64_SHA/" sync-app.rb
fi

# Update version
sed -i "s/version \".*\"/version \"$VERSION\"/" sync-app.rb
sed -i "s/v0\.1\.0/v$VERSION/g" sync-app.rb

cp sync-app.rb "../../$OUTPUT_DIR/"

# Restore file
git checkout sync-app.rb 2>/dev/null || true

cd ../..

# Copy release archives to packages directory
echo ""
echo "Copying release archives..."
cp target/release-builds/*.tar.gz "$OUTPUT_DIR/" 2>/dev/null || true
cp target/release-builds/*.zip "$OUTPUT_DIR/" 2>/dev/null || true
cp target/release-builds/checksums-*.txt "$OUTPUT_DIR/" 2>/dev/null || true

echo ""
echo "Package build complete! Artifacts available in: $OUTPUT_DIR"
echo ""
echo "Files created:"
ls -la "$OUTPUT_DIR"

echo ""
echo "Package Summary:"
echo "================"

if ls "$OUTPUT_DIR"/*.deb >/dev/null 2>&1; then
    echo "✓ Debian packages (.deb)"
fi

if ls "$OUTPUT_DIR"/*.rpm >/dev/null 2>&1; then
    echo "✓ RPM packages (.rpm)"
fi

if ls "$OUTPUT_DIR"/*.nupkg >/dev/null 2>&1; then
    echo "✓ Chocolatey package (.nupkg)"
fi

if ls "$OUTPUT_DIR"/sync-app.rb >/dev/null 2>&1; then
    echo "✓ Homebrew formula (.rb)"
fi

if ls "$OUTPUT_DIR"/*.tar.gz >/dev/null 2>&1; then
    echo "✓ Binary archives (.tar.gz, .zip)"
fi

echo ""
echo "Next steps:"
echo "1. Test packages on target systems"
echo "2. Upload to package repositories"
echo "3. Create GitHub release: scripts/create-release.sh $VERSION"
echo "4. Submit Homebrew formula to homebrew-core"
echo "5. Publish Chocolatey package to community repository"
