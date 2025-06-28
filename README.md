# Sync App

A multi-crate Rust workspace for synchronization with PocketBase backend.

## Project Structure

```
sync-app/
├── sync-core/          # Core synchronization library
├── sync-cli/           # Command line interface
├── sync-server/        # Server component
├── pocketbase/         # PocketBase binaries and data
├── .github/workflows/  # GitHub Actions CI
└── ...
```

## Prerequisites

- Rust (stable toolchain)
- Git

## Getting Started

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd sync-app
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test
   ```

4. Set up PocketBase:
   ```bash
   # The build script will automatically download PocketBase binaries
   # Or you can add PocketBase as a Git submodule:
   git submodule add https://github.com/pocketbase/pocketbase.git pocketbase-source
   ```

## Development

### Formatting
```bash
cargo fmt
```

### Linting
```bash
cargo clippy
```

### Coverage
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --all-features --workspace
```

## License

This project is licensed under the GNU Affero General Public License v3.0. See the [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## Architecture

- **sync-core**: Contains the core synchronization logic, data structures, and PocketBase client
- **sync-cli**: Command-line interface for interacting with the sync system
- **sync-server**: Server component that can run as a daemon
- **pocketbase/**: Contains PocketBase binaries and runtime data

## PocketBase Integration

This project integrates with PocketBase for backend data storage and synchronization. The build script automatically downloads the appropriate PocketBase binary for your platform.

PocketBase data is stored in `pocketbase/pb_data/` (excluded from git).
