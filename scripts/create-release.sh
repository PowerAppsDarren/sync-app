#!/bin/bash
set -euo pipefail

# GitHub Release Creation Script
# This script creates a GitHub release with binaries, checksums, and changelog

VERSION=${1:-$(grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}
OUTPUT_DIR="target/release-builds"
REPO_URL=${REPO_URL:-$(git config --get remote.origin.url | sed 's/\.git$//')}
REPO_NAME=${REPO_NAME:-$(basename "$REPO_URL")}
REPO_OWNER=${REPO_OWNER:-$(dirname "$REPO_URL" | xargs basename)}

echo "Creating GitHub release for sync-app v${VERSION}"
echo "Repository: ${REPO_OWNER}/${REPO_NAME}"

# Check if gh CLI is available
if ! command -v gh >/dev/null 2>&1; then
    echo "Error: GitHub CLI (gh) is required but not installed."
    echo "Please install it from: https://cli.github.com/"
    exit 1
fi

# Check if we're authenticated
if ! gh auth status >/dev/null 2>&1; then
    echo "Error: Not authenticated with GitHub CLI."
    echo "Please run: gh auth login"
    exit 1
fi

# Check if release artifacts exist
if [ ! -d "$OUTPUT_DIR" ]; then
    echo "Error: Release artifacts not found in $OUTPUT_DIR"
    echo "Please run: scripts/build-release.sh first"
    exit 1
fi

# Generate or update changelog
generate_changelog() {
    local changelog_file="CHANGELOG.md"
    local temp_changelog=$(mktemp)
    
    echo "Generating changelog for v${VERSION}..."
    
    # Get the previous tag
    local previous_tag=$(git tag --sort=-version:refname | head -n 1 2>/dev/null || echo "")
    
    echo "## [${VERSION}] - $(date +%Y-%m-%d)" > "$temp_changelog"
    echo "" >> "$temp_changelog"
    
    if [ -n "$previous_tag" ]; then
        echo "### Changes since ${previous_tag}" >> "$temp_changelog"
        echo "" >> "$temp_changelog"
        
        # Generate commit messages since last tag
        git log --pretty=format:"- %s" "${previous_tag}..HEAD" >> "$temp_changelog" 2>/dev/null || {
            echo "- Initial release" >> "$temp_changelog"
        }
    else
        echo "### Initial Release" >> "$temp_changelog"
        echo "" >> "$temp_changelog"
        echo "- First release of sync-app" >> "$temp_changelog"
        echo "- Cross-platform synchronization with PocketBase backend" >> "$temp_changelog"
        echo "- CLI tool, server component, and daemon service" >> "$temp_changelog"
        echo "- Support for Linux, macOS, and Windows" >> "$temp_changelog"
    fi
    
    echo "" >> "$temp_changelog"
    echo "### Downloads" >> "$temp_changelog"
    echo "" >> "$temp_changelog"
    echo "Choose the appropriate binary for your platform:" >> "$temp_changelog"
    echo "" >> "$temp_changelog"
    echo "- **Linux x86_64 (MUSL)**: \`sync-app-${VERSION}-x86_64-unknown-linux-musl.tar.gz\`" >> "$temp_changelog"
    echo "- **Linux ARM64 (MUSL)**: \`sync-app-${VERSION}-aarch64-unknown-linux-musl.tar.gz\`" >> "$temp_changelog"
    echo "- **Linux x86_64 (GNU)**: \`sync-app-${VERSION}-x86_64-unknown-linux-gnu.tar.gz\`" >> "$temp_changelog"
    echo "- **Linux ARM64 (GNU)**: \`sync-app-${VERSION}-aarch64-unknown-linux-gnu.tar.gz\`" >> "$temp_changelog"
    echo "- **Windows x86_64**: \`sync-app-${VERSION}-x86_64-pc-windows-gnu.zip\`" >> "$temp_changelog"
    echo "- **macOS x86_64**: \`sync-app-${VERSION}-x86_64-apple-darwin.tar.gz\`" >> "$temp_changelog"
    echo "- **macOS ARM64**: \`sync-app-${VERSION}-aarch64-apple-darwin.tar.gz\`" >> "$temp_changelog"
    echo "" >> "$temp_changelog"
    echo "All downloads include the PocketBase binary as an optional dependency." >> "$temp_changelog"
    echo "" >> "$temp_changelog"
    echo "### Verification" >> "$temp_changelog"
    echo "" >> "$temp_changelog"
    echo "Verify your download using the provided checksums:" >> "$temp_changelog"
    echo "\`\`\`bash" >> "$temp_changelog"
    echo "sha256sum -c checksums-${VERSION}.txt" >> "$temp_changelog"
    echo "\`\`\`" >> "$temp_changelog"
    
    # If changelog exists, prepend new content
    if [ -f "$changelog_file" ]; then
        echo "" >> "$temp_changelog"
        cat "$changelog_file" >> "$temp_changelog"
    fi
    
    mv "$temp_changelog" "$changelog_file"
    echo "Changelog updated: $changelog_file"
}

# Create the GitHub release
create_github_release() {
    echo "Creating GitHub release..."
    
    # Extract release notes for this version
    local release_notes=$(mktemp)
    local in_version=false
    
    while IFS= read -r line; do
        if [[ "$line" =~ ^\#\#[[:space:]]\[${VERSION}\] ]]; then
            in_version=true
            continue
        elif [[ "$line" =~ ^\#\#[[:space:]] ]] && [ "$in_version" = true ]; then
            break
        elif [ "$in_version" = true ]; then
            echo "$line" >> "$release_notes"
        fi
    done < CHANGELOG.md
    
    # Create the release
    gh release create "v${VERSION}" \
        --title "Release v${VERSION}" \
        --notes-file "$release_notes" \
        --draft=false \
        --prerelease=false
    
    rm -f "$release_notes"
}

# Upload release assets
upload_assets() {
    echo "Uploading release assets..."
    
    cd "$OUTPUT_DIR"
    
    # Upload all archives
    for file in *.tar.gz *.zip; do
        if [ -f "$file" ]; then
            echo "Uploading: $file"
            gh release upload "v${VERSION}" "$file"
        fi
    done
    
    # Upload checksums
    if [ -f "checksums-${VERSION}.txt" ]; then
        echo "Uploading: checksums-${VERSION}.txt"
        gh release upload "v${VERSION}" "checksums-${VERSION}.txt"
    fi
    
    cd - >/dev/null
}

# Main release process
echo "Starting GitHub release process..."

# Generate changelog
generate_changelog

# Commit changelog if it changed
if git diff --quiet CHANGELOG.md; then
    echo "No changes to changelog"
else
    echo "Committing changelog..."
    git add CHANGELOG.md
    git commit -m "Update changelog for v${VERSION}"
fi

# Create and push tag
if git rev-parse "v${VERSION}" >/dev/null 2>&1; then
    echo "Tag v${VERSION} already exists"
else
    echo "Creating tag v${VERSION}..."
    git tag -a "v${VERSION}" -m "Release v${VERSION}"
    git push origin "v${VERSION}"
fi

# Create GitHub release
create_github_release

# Upload assets
upload_assets

echo "----------------------------------------"
echo "GitHub release v${VERSION} created successfully!"
echo ""
echo "Release URL: ${REPO_URL}/releases/tag/v${VERSION}"
echo ""
echo "Next steps:"
echo "1. Update package managers (Homebrew, Chocolatey, etc.)"
echo "2. Update documentation"
echo "3. Announce the release"
