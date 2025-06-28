#!/bin/bash

# Setup script for PocketBase
# This script downloads and sets up PocketBase for development

set -e

POCKETBASE_VERSION="0.22.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect platform
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="linux_amd64"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ "$(uname -m)" == "arm64" ]]; then
        PLATFORM="darwin_arm64"
    else
        PLATFORM="darwin_amd64"
    fi
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    PLATFORM="windows_amd64"
else
    echo "Unsupported platform: $OSTYPE"
    exit 1
fi

FILENAME="pocketbase_${POCKETBASE_VERSION}_${PLATFORM}.zip"
URL="https://github.com/pocketbase/pocketbase/releases/download/v${POCKETBASE_VERSION}/${FILENAME}"

echo "Setting up PocketBase ${POCKETBASE_VERSION} for ${PLATFORM}..."

# Download PocketBase if not already present
if [ ! -f "${SCRIPT_DIR}/${FILENAME}" ]; then
    echo "Downloading ${FILENAME}..."
    curl -L -o "${SCRIPT_DIR}/${FILENAME}" "${URL}"
else
    echo "PocketBase archive already exists, skipping download."
fi

# Extract PocketBase
echo "Extracting PocketBase..."
unzip -o "${SCRIPT_DIR}/${FILENAME}" -d "${SCRIPT_DIR}"

# Make executable (Unix systems)
if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "win32" ]]; then
    chmod +x "${SCRIPT_DIR}/pocketbase"
fi

# Create pb_data directory for PocketBase data
mkdir -p "${SCRIPT_DIR}/pb_data"

echo "PocketBase setup complete!"
echo "You can start PocketBase with:"
echo "  cd pocketbase && ./pocketbase serve"
echo ""
echo "PocketBase admin UI will be available at: http://localhost:8090/_/"
