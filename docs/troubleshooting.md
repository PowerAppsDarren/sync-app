# Troubleshooting & FAQ

This guide helps you diagnose and resolve common issues with Sync App. It covers installation problems, runtime errors, performance issues, and frequently asked questions.

## Quick Diagnostics

### Health Check Commands

Run these commands to quickly assess system health:

```bash
# Check daemon status
sync-daemon status

# Verify PocketBase connectivity
curl -f http://localhost:8090/api/health

# Check metrics endpoint
curl -f http://localhost:9090/metrics

# View recent logs
tail -n 50 /var/log/sync-daemon.log

# Test configuration
sync-daemon config validate --config /path/to/daemon.toml
```

### Log Locations

- **Daemon logs**: `/var/log/sync-daemon.log` (Linux), `logs/daemon.log` (Windows)
- **PocketBase logs**: PocketBase data directory, usually `/opt/sync-app/data/logs/`
- **System logs**: `journalctl -u sync-daemon` (systemd), Event Viewer (Windows)

## Common Issues

### Installation & Setup Issues

#### 1. "Command not found: sync" or "Command not found: daemon"

**Problem**: Binaries are not in PATH or not installed correctly.

**Solutions**:
```bash
# Check if binaries exist
ls -la target/release/sync target/release/daemon

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$PATH:/path/to/sync-app/target/release"

# Or create symlinks
sudo ln -s /path/to/sync-app/target/release/sync /usr/local/bin/
sudo ln -s /path/to/sync-app/target/release/daemon /usr/local/bin/

# Verify installation
which sync daemon
sync --version
daemon --version
```

#### 2. Rust Compilation Errors

**Problem**: Build fails with dependency or compilation errors.

**Solutions**:
```bash
# Update Rust toolchain
rustup update

# Clean build cache
cargo clean

# Check Rust version (requires 1.70+)
rustc --version

# Install required system dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# Install required system dependencies (CentOS/RHEL)
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# Rebuild
cargo build --release
```

#### 3. PocketBase Download/Setup Issues

**Problem**: PocketBase setup script fails or binary is not found.

**Solutions**:
```bash
# Manual PocketBase download (Linux x64)
cd pocketbase
wget https://github.com/pocketbase/pocketbase/releases/download/v0.22.0/pocketbase_0.22.0_linux_amd64.zip
unzip pocketbase_0.22.0_linux_amd64.zip
chmod +x pocketbase

# For other platforms, check: https://pocketbase.io/docs/
# Windows: pocketbase_0.22.0_windows_amd64.zip
# macOS: pocketbase_0.22.0_darwin_amd64.zip

# Test PocketBase
./pocketbase --help
./pocketbase serve --http=localhost:8090
```

### Connection Issues

#### 4. "Connection refused" to PocketBase

**Problem**: Cannot connect to PocketBase server.

**Diagnosis**:
```bash
# Check if PocketBase is running
ps aux | grep pocketbase
sudo netstat -tlnp | grep :8090

# Check firewall (Ubuntu)
sudo ufw status

# Check firewall (CentOS/RHEL)
sudo firewall-cmd --list-all
```

**Solutions**:
```bash
# Start PocketBase manually
cd pocketbase
./pocketbase serve --http=0.0.0.0:8090

# Check configuration
grep -A 5 "\[pocketbase\]" daemon.toml

# Test connectivity
curl -v http://localhost:8090/api/health
telnet localhost 8090

# For Docker, check port mapping
docker ps
docker logs container-name
```

#### 5. PocketBase Authentication Failures

**Problem**: "401 Unauthorized" or "Invalid credentials" errors.

**Solutions**:
```bash
# Verify admin credentials
curl -X POST http://localhost:8090/api/admins/auth-with-password \
  -H "Content-Type: application/json" \
  -d '{"identity":"admin@example.com","password":"admin123456"}'

# Reset PocketBase admin (if needed)
cd pocketbase
./pocketbase admin create admin@example.com admin123456

# Check configuration
cat daemon.toml | grep -A 3 "\[pocketbase\]"

# Ensure URL format is correct (no trailing slash)
# Correct: http://localhost:8090
# Incorrect: http://localhost:8090/
```

### Runtime Issues

#### 6. Daemon Crashes or Exits Unexpectedly

**Problem**: Daemon process terminates without clear reason.

**Diagnosis**:
```bash
# Check exit status
echo $?

# View detailed logs
journalctl -u sync-daemon -f --no-pager

# Check system resources
free -h
df -h
ps aux --sort=-%mem | head -10

# Check for core dumps
coredumpctl list
```

**Solutions**:
```bash
# Increase verbosity
# In daemon.toml:
[daemon]
log_level = "debug"

# Or set environment variable
RUST_LOG=debug sync-daemon start

# Adjust memory limits if needed
# In systemd service:
[Service]
MemoryMax=2G

# Check for file descriptor limits
ulimit -n
# Increase if needed:
ulimit -n 65536
```

#### 7. High Memory Usage

**Problem**: Daemon consumes excessive memory.

**Diagnosis**:
```bash
# Monitor memory usage
ps aux | grep daemon
top -p $(pgrep daemon)

# Check configuration
grep -A 10 "\[concurrency\]" daemon.toml
```

**Solutions**:
```toml
# Reduce concurrency in daemon.toml
[concurrency]
max_concurrent_syncs = 2        # Reduce from default
max_file_operations = 50        # Reduce from default
sync_queue_size = 500          # Reduce from default

[cache]
max_cache_size_mb = 100        # Reduce cache size
enable_persistent_cache = false # Disable if not needed
```

#### 8. Sync Operations Fail

**Problem**: Individual sync jobs fail with various errors.

**Common Error Messages and Solutions**:

**"Permission denied"**:
```bash
# Check file permissions
ls -la /path/to/source /path/to/destination

# Fix ownership
sudo chown -R sync-user:sync-group /path/to/directories

# Check daemon user permissions
sudo -u sync-app ls /path/to/source
```

**"No such file or directory"**:
```bash
# Verify paths exist
ls -la ~/Documents
ls -la /backup/documents

# Check path expansion
echo ~/Documents  # Should show full path

# Use absolute paths in configuration
source_path = "/home/user/Documents"  # Not ~/Documents
```

**"Disk space insufficient"**:
```bash
# Check disk space
df -h

# Clean up space
sudo apt autoremove && sudo apt autoclean  # Ubuntu
sudo yum clean all  # CentOS

# Adjust sync options
[sync_jobs.sync_options]
max_file_size_mb = 100  # Skip large files
```

### Performance Issues

#### 9. Slow Sync Performance

**Problem**: Synchronization takes longer than expected.

**Diagnosis**:
```bash
# Check network latency
ping your-pocketbase-server

# Monitor I/O
iostat -x 1 5

# Check current operations
curl -s http://localhost:9090/metrics | grep sync_operations

# View detailed timing
RUST_LOG=sync=debug sync-daemon start
```

**Solutions**:
```toml
# Optimize configuration
[concurrency]
max_concurrent_syncs = 8        # Increase for better throughput
max_file_operations = 200       # Increase for large filesets

[sync_jobs.sync_options]
comparison_method = "xxhash"    # Faster than sha256
verify_checksums = false       # Skip verification for speed
compression_enabled = true     # May help with network transfer

[cache]
enable_persistent_cache = true # Avoid recalculating hashes
file_metadata_cache_ttl_secs = 300  # Cache longer
```

#### 10. High CPU Usage

**Problem**: Daemon uses excessive CPU resources.

**Solutions**:
```toml
# Reduce file scanning frequency
[file_watchers]
debounce_ms = 5000             # Increase debounce time

# Limit concurrent operations
[concurrency]
max_concurrent_syncs = 2
thread_pool_size = 4           # Limit thread pool

# Use lighter comparison method
[sync_jobs.sync_options]
comparison_method = "mtime"    # Faster than hash-based
```

### Configuration Issues

#### 11. Invalid Configuration File

**Problem**: Daemon fails to start due to configuration errors.

**Diagnosis**:
```bash
# Validate configuration
sync-daemon config validate --config daemon.toml

# Check TOML syntax
python3 -c "import toml; toml.load('daemon.toml')"
```

**Solutions**:
```bash
# Common TOML syntax errors:

# Missing quotes around strings
url = http://localhost:8090     # Wrong
url = "http://localhost:8090"   # Correct

# Invalid array syntax
filters = *.tmp, *.log          # Wrong  
filters = ["*.tmp", "*.log"]    # Correct

# Missing required fields
# Each sync_job needs: id, name, source_path, destination_path
# Each schedule needs: type, and either interval or expression
```

#### 12. Schedule Not Working

**Problem**: Scheduled sync jobs don't run as expected.

**Diagnosis**:
```bash
# Check scheduler status
curl -s http://localhost:9090/metrics | grep scheduler

# View scheduler logs
grep -i schedule /var/log/sync-daemon.log
```

**Solutions**:
```toml
# Verify schedule configuration
[sync_jobs.schedule]
type = "interval"
interval = "5m"               # Valid: 30s, 5m, 1h, 2d

# For cron schedules
type = "cron"
expression = "0 */2 * * *"    # Every 2 hours
timezone = "America/New_York" # Use valid timezone

# Enable job
enabled = true                # Check this is set
```

### File Watcher Issues

#### 13. File Watcher Not Triggering

**Problem**: File changes don't trigger automatic syncs.

**Diagnosis**:
```bash
# Check watcher status
curl -s http://localhost:9090/metrics | grep watcher

# Test manual file operations
touch /watched/path/test.txt
echo "test" >> /watched/path/test.txt
rm /watched/path/test.txt

# Check system limits
cat /proc/sys/fs/inotify/max_user_watches
```

**Solutions**:
```bash
# Increase inotify limits (Linux)
echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

# Verify watcher configuration
[file_watchers]
enabled = true
recursive = true
watch_events = ["create", "write", "remove"]
debounce_ms = 1000

# Check path permissions
sudo -u sync-app ls -la /watched/path
```

## Advanced Troubleshooting

### Debug Mode

Enable comprehensive debugging:

```bash
# Environment variable method
RUST_LOG=debug,sync_daemon=trace sync-daemon start --config daemon.toml

# Configuration method (in daemon.toml)
[daemon]
log_level = "trace"

[telemetry]
log_level = "debug"
json_logging = true
```

### Performance Profiling

Profile daemon performance:

```bash
# Install profiling tools
cargo install flamegraph

# Profile the daemon
sudo perf record -g target/release/daemon start --config daemon.toml
sudo perf script | flamegraph.pl > flame.svg

# Memory profiling with valgrind
valgrind --tool=massif target/release/daemon start --config daemon.toml
```

### Network Debugging

Diagnose network issues:

```bash
# Monitor network traffic
sudo tcpdump -i any port 8090

# Trace HTTP requests
curl -v -X GET http://localhost:8090/api/health

# Check DNS resolution
nslookup your-pocketbase-server
dig your-pocketbase-server

# Test with different tools
wget -O- http://localhost:8090/api/health
nc -zv localhost 8090
```

## Frequently Asked Questions

### General Questions

**Q: Can I run multiple sync jobs simultaneously?**
A: Yes, the daemon supports concurrent sync jobs. Configure `max_concurrent_syncs` in the `[concurrency]` section.

**Q: How do I backup my configurations?**
A: Use the CLI export command:
```bash
sync export backup-configs.json
```

**Q: Can I sync to cloud storage (S3, Google Drive, etc.)?**
A: Currently, Sync App supports local filesystem paths only. Cloud storage support may be added in future versions.

**Q: Is bidirectional sync supported?**
A: Yes, set `bidirectional = true` in sync options. Configure conflict resolution strategy for handling conflicts.

**Q: How do I upgrade Sync App?**
A: Stop the daemon, backup configurations, build/install new version, migrate configs if needed, restart daemon.

### Performance Questions

**Q: What's the maximum number of files Sync App can handle?**
A: There's no hard limit, but performance depends on system resources. Monitor memory usage and adjust concurrency settings.

**Q: How can I improve sync performance?**
A: Use SSD storage, increase concurrency settings, use faster comparison methods (xxhash), enable caching.

**Q: Does Sync App support resume for interrupted transfers?**
A: Yes, interrupted transfers are automatically resumed on the next sync operation.

### Security Questions

**Q: Are files encrypted during transfer?**
A: Files are transferred securely if you use HTTPS for PocketBase. Local transfers are not encrypted.

**Q: How are credentials stored?**
A: Credentials are stored in plain text in configuration files. Secure file permissions are recommended.

**Q: Can I use custom SSL certificates?**
A: Yes, configure your reverse proxy (nginx) or PocketBase to use custom certificates.

### Deployment Questions

**Q: Can I run Sync App in Docker?**
A: Yes, see the [deployment guide](deployment.md) for Docker configurations.

**Q: How do I monitor Sync App in production?**
A: Use the Prometheus metrics endpoint (`/metrics`) with Grafana dashboards. Enable structured logging for log aggregation.

**Q: What happens if PocketBase goes down?**
A: Sync operations will fail and retry automatically. Local file operations can continue if not dependent on PocketBase.

## Getting Additional Help

If you're still experiencing issues:

1. **Enable debug logging** and collect relevant log snippets
2. **Check the GitHub Issues** for similar problems
3. **Create a new issue** with:
   - Sync App version (`sync --version`)
   - Operating system and version
   - Configuration file (remove sensitive data)
   - Complete error messages
   - Steps to reproduce the issue

### Useful Commands for Bug Reports

```bash
# Collect system information
uname -a
cargo --version
rustc --version

# Generate configuration summary (remove sensitive data)
sync-daemon config validate --config daemon.toml --verbose

# Collect recent logs
tail -n 100 /var/log/sync-daemon.log

# Export metrics
curl -s http://localhost:9090/metrics > metrics.txt

# Test basic functionality
sync health --verbose
```

### Community Resources

- **Documentation**: [https://yourusername.github.io/sync-app/](https://yourusername.github.io/sync-app/)
- **GitHub Issues**: [https://github.com/yourusername/sync-app/issues](https://github.com/yourusername/sync-app/issues)
- **Discussions**: [https://github.com/yourusername/sync-app/discussions](https://github.com/yourusername/sync-app/discussions)
- **Matrix Chat**: `#sync-app:matrix.org` (if available)

Remember to always backup your data and test configurations in development environments before deploying to production!
