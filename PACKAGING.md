# Packaging & Distribution

This document describes the comprehensive packaging and distribution system for sync-app, implementing cross-platform binary builds, package management integration, and automated releases.

## Overview

The packaging system provides:

- **Cross-platform binaries**: MUSL and GNU Linux, Windows, and macOS (x86_64 + ARM64)
- **PocketBase bundling**: Optional PocketBase binary included with all distributions
- **Package managers**: Homebrew, Chocolatey, Debian (.deb), and RPM packages
- **Service integration**: systemd units for Linux, Windows services, and launchd for macOS
- **Automated releases**: GitHub Actions workflow with checksums and changelogs

## Quick Start

### Build All Packages

```bash
# Build cross-platform binaries and all package formats
./scripts/package-all.sh

# Or build specific components
./scripts/build-release.sh              # Cross-platform binaries only
./packaging/debian/build-deb.sh         # Debian packages
./packaging/rpm/build-rpm.sh            # RPM packages
```

### Create GitHub Release

```bash
# Build and create a GitHub release
./scripts/build-release.sh
./scripts/create-release.sh
```

## Build System

### Cross Compilation

The build system uses [cross](https://github.com/cross-rs/cross) for reliable cross-compilation:

**Supported targets:**
- `x86_64-unknown-linux-musl` - Linux x86_64 (static, portable)
- `aarch64-unknown-linux-musl` - Linux ARM64 (static, portable)  
- `x86_64-unknown-linux-gnu` - Linux x86_64 (dynamic linking)
- `aarch64-unknown-linux-gnu` - Linux ARM64 (dynamic linking)
- `x86_64-pc-windows-gnu` - Windows x86_64
- `x86_64-apple-darwin` - macOS x86_64 (Intel)
- `aarch64-apple-darwin` - macOS ARM64 (Apple Silicon)

**Configuration:** `Cross.toml`

### PocketBase Integration

Each release includes the appropriate PocketBase binary for the target platform:

- Downloaded automatically during build
- Version controlled via `POCKETBASE_VERSION` environment variable
- Bundled as optional dependency in all packages

## Package Formats

### Homebrew Formula

**Location:** `packaging/homebrew/sync-app.rb`

**Features:**
- Multi-platform support (macOS, Linux)
- Architecture detection (x86_64, ARM64)
- Service management via `brew services`
- Shell completion generation
- Automatic dependency handling

**Installation:**
```bash
brew install sync-app
brew services start sync-app
```

### Chocolatey Package

**Location:** `packaging/chocolatey/`

**Features:**
- Windows x86_64 support
- PowerShell installation scripts
- Windows service integration
- PATH management
- Configuration directory setup

**Installation:**
```powershell
choco install sync-app
```

### Debian Packages

**Location:** `packaging/debian/`

**Features:**
- Multi-architecture (amd64, arm64)
- systemd service integration
- User/group management
- Configuration preservation
- Proper dependency handling

**Installation:**
```bash
sudo dpkg -i sync-app_0.1.0_amd64.deb
sudo systemctl enable --now sync-daemon
```

### RPM Packages

**Location:** `packaging/rpm/`

**Features:**
- Multi-architecture (x86_64, aarch64)
- systemd service integration
- User/group management
- Security-hardened service configuration
- Proper scriptlet handling

**Installation:**
```bash
sudo rpm -ivh sync-app-0.1.0-1.x86_64.rpm
sudo systemctl enable --now sync-daemon
```

## Service Configuration

### Linux (systemd)

**Service file:** `/lib/systemd/system/sync-daemon.service`

**Features:**
- Security hardening (NoNewPrivileges, PrivateTmp, etc.)
- Automatic restart on failure
- Proper user/group isolation
- Resource protection

**Management:**
```bash
sudo systemctl start sync-daemon
sudo systemctl enable sync-daemon
sudo systemctl status sync-daemon
```

### Windows Service

**Features:**
- Windows service integration via `sync-daemon install`
- Automatic startup configuration
- Service management via PowerShell or Services.msc

**Management:**
```powershell
sync-daemon install
sync-daemon start
Get-Service sync-daemon
```

### macOS (launchd)

**Features:**
- Homebrew service integration
- User-level daemon support
- Automatic startup management

**Management:**
```bash
brew services start sync-app
brew services stop sync-app
```

## Automated Releases

### GitHub Actions Workflow

**File:** `.github/workflows/release.yml`

**Triggers:**
- Git tags matching `v*` pattern
- Manual workflow dispatch

**Process:**
1. **Build job**: Cross-compile for all targets, bundle PocketBase
2. **Package job**: Create all package formats (.deb, .rpm, etc.)
3. **Release job**: Generate release notes, create GitHub release with assets

**Artifacts:**
- Binary archives (`.tar.gz`, `.zip`)
- Package files (`.deb`, `.rpm`, `.nupkg`)
- Checksums file
- Updated Homebrew formula

### Manual Release Process

1. **Update version** in `Cargo.toml` files
2. **Commit and tag**:
   ```bash
   git add .
   git commit -m "Release v0.1.0"
   git tag v0.1.0
   git push origin main --tags
   ```
3. **GitHub Actions will automatically**:
   - Build all platforms
   - Create packages
   - Generate release with notes
   - Upload all assets

## Directory Structure

```
packaging/
├── homebrew/           # Homebrew formula
│   └── sync-app.rb
├── chocolatey/         # Chocolatey package
│   ├── sync-app.nuspec
│   └── tools/
│       ├── chocolateyinstall.ps1
│       └── chocolateyuninstall.ps1
├── debian/             # Debian packages
│   └── build-deb.sh
└── rpm/                # RPM packages
    ├── sync-app.spec
    └── build-rpm.sh

scripts/
├── build-release.sh    # Cross-platform binary builds
├── create-release.sh   # GitHub release creation
└── package-all.sh      # Build all package formats

.github/workflows/
└── release.yml         # Automated release workflow
```

## Configuration

### Default Locations

**Linux:**
- System config: `/etc/sync-app/config.yaml`
- User data: `/var/lib/sync-app/`
- Logs: `/var/log/sync-app/`

**Windows:**
- System config: `%APPDATA%\sync-app\config.yaml`
- User data: `%APPDATA%\sync-app\`
- Logs: `%APPDATA%\sync-app\sync.log`

**macOS:**
- System config: `/opt/homebrew/etc/sync-app/config.yaml`
- User data: `/opt/homebrew/var/lib/sync-app/`
- Logs: `/opt/homebrew/var/log/sync-app.log`

### Default Configuration

```yaml
server:
  host: "127.0.0.1"
  port: 8080
  
database:
  path: "./sync.db"
  
logging:
  level: "info"
  file: "./sync.log"
  
sync:
  interval: "30s"
  auto_start: false
```

## Security

### Service Hardening

All service configurations include security hardening:

- **User isolation**: Dedicated `sync-app` user/group
- **Filesystem protection**: Read-only root, restricted paths
- **Capability restrictions**: Minimal required capabilities
- **Network isolation**: No unnecessary network access
- **Resource limits**: Memory and process limits

### Package Verification

All releases include SHA256 checksums:

```bash
# Verify download integrity
sha256sum -c checksums-0.1.0.txt
```

## Troubleshooting

### Build Issues

1. **Cross compilation fails**: Ensure Docker is running for cross
2. **PocketBase download fails**: Check network connectivity and version
3. **Package build fails**: Verify required tools are installed

### Package Issues

1. **Service won't start**: Check configuration file syntax
2. **Permission errors**: Verify user/group creation during install
3. **Missing dependencies**: Use package manager to install dependencies

### Common Commands

```bash
# Rebuild all packages
./scripts/package-all.sh

# Test package installation
sudo dpkg -i target/packages/sync-app_0.1.0_amd64.deb
sudo systemctl status sync-daemon

# Clean build artifacts
rm -rf target/release-builds target/packages

# Update to new version
# 1. Update Cargo.toml version
# 2. Run package-all.sh
# 3. Test packages
# 4. Create release
```

## Contributing

When adding new package formats or modifying the build system:

1. **Update documentation** in this file
2. **Test on target platforms** before merging
3. **Follow existing patterns** for consistency
4. **Add appropriate error handling** in scripts
5. **Update GitHub Actions** if needed

## Future Enhancements

- **Snap packages** for Linux
- **MSI installer** for Windows
- **DMG packages** for macOS
- **Docker images** for containerized deployment
- **Kubernetes manifests** for cloud deployment
- **Package repository hosting** for direct installation
