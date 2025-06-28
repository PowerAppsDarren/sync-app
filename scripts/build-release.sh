#!/bin/bash
set -euo pipefail

# Build script for cross-platform releases
# This script builds binaries for multiple platforms using cross or cargo

VERSION=${1:-$(grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}
OUTPUT_DIR="target/release-builds"
POCKETBASE_VERSION=${POCKETBASE_VERSION:-"0.20.1"}

echo "Building sync-app v${VERSION}"
echo "PocketBase version: ${POCKETBASE_VERSION}"

# Clean and create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Define target platforms
TARGETS=(
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu" 
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

# Function to build for a target
build_target() {
    local target=$1
    echo "Building for target: $target"
    
    # Determine if we should use cross or cargo
    if command -v cross >/dev/null 2>&1; then
        BUILDER="cross"
    else
        BUILDER="cargo"
        echo "Warning: cross not found, falling back to cargo. Install cross for better cross-compilation support."
    fi
    
    # Build the main binaries
    $BUILDER build --release --target "$target" --bin sync --bin sync-server --bin sync-daemon
    
    # Create target-specific directory
    local target_dir="$OUTPUT_DIR/$target"
    mkdir -p "$target_dir"
    
    # Copy binaries
    local bin_ext=""
    if [[ "$target" == *"windows"* ]]; then
        bin_ext=".exe"
    fi
    
    cp "target/$target/release/sync$bin_ext" "$target_dir/"
    cp "target/$target/release/sync-server$bin_ext" "$target_dir/"
    cp "target/$target/release/sync-daemon$bin_ext" "$target_dir/"
    
    # Download and bundle PocketBase if needed
    download_pocketbase "$target" "$target_dir"
    
    # Create archive
    create_archive "$target" "$target_dir"
}

# Function to download PocketBase for the target
download_pocketbase() {
    local target=$1
    local target_dir=$2
    
    echo "Downloading PocketBase for $target"
    
    # Map Rust targets to PocketBase download names
    local pb_os=""
    local pb_arch=""
    local pb_ext=""
    
    case "$target" in
        *"linux"*)
            pb_os="linux"
            pb_ext=""
            ;;
        *"windows"*)
            pb_os="windows"
            pb_ext=".exe"
            ;;
        *"darwin"*)
            pb_os="darwin"
            pb_ext=""
            ;;
        *)
            echo "Warning: Unknown OS for target $target, skipping PocketBase"
            return
            ;;
    esac
    
    case "$target" in
        *"x86_64"*)
            pb_arch="amd64"
            ;;
        *"aarch64"*)
            pb_arch="arm64"
            ;;
        *)
            echo "Warning: Unknown architecture for target $target, skipping PocketBase"
            return
            ;;
    esac
    
    local pb_filename="pocketbase_${POCKETBASE_VERSION}_${pb_os}_${pb_arch}.zip"
    local pb_url="https://github.com/pocketbase/pocketbase/releases/download/v${POCKETBASE_VERSION}/${pb_filename}"
    
    echo "Downloading: $pb_url"
    
    # Download and extract PocketBase
    local temp_dir=$(mktemp -d)
    cd "$temp_dir"
    
    if curl -L -o "$pb_filename" "$pb_url"; then
        unzip -q "$pb_filename"
        cp "pocketbase$pb_ext" "$target_dir/"
        echo "PocketBase bundled successfully"
    else
        echo "Warning: Failed to download PocketBase for $target"
    fi
    
    cd - >/dev/null
    rm -rf "$temp_dir"
}

# Function to create archives
create_archive() {
    local target=$1
    local target_dir=$2
    
    echo "Creating archive for $target"
    
    cd "$OUTPUT_DIR"
    
    # Create compressed archive
    if [[ "$target" == *"windows"* ]]; then
        # Use zip for Windows
        zip -r "sync-app-${VERSION}-${target}.zip" "${target}/"
    else
        # Use tar.gz for Unix-like systems
        tar czf "sync-app-${VERSION}-${target}.tar.gz" "${target}/"
    fi
    
    cd - >/dev/null
}

# Function to generate checksums
generate_checksums() {
    echo "Generating checksums..."
    cd "$OUTPUT_DIR"
    
    # Generate SHA256 checksums
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum *.zip *.tar.gz > "checksums-${VERSION}.txt" 2>/dev/null || true
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 *.zip *.tar.gz > "checksums-${VERSION}.txt" 2>/dev/null || true
    else
        echo "Warning: No checksum utility found"
    fi
    
    cd - >/dev/null
}

# Main build process
echo "Starting cross-platform build process..."

# Install cross if not available and cargo is available
if ! command -v cross >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    echo "Installing cross..."
    cargo install cross --git https://github.com/cross-rs/cross
fi

# Build for each target
for target in "${TARGETS[@]}"; do
    echo "----------------------------------------"
    build_target "$target"
done

# Generate checksums
generate_checksums

echo "----------------------------------------"
echo "Build complete! Artifacts available in: $OUTPUT_DIR"
echo "Files created:"
ls -la "$OUTPUT_DIR"

echo ""
echo "To create a GitHub release, run:"
echo "  scripts/create-release.sh $VERSION"
