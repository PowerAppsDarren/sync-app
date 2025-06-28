# Production Deployment Guide

This guide covers deploying Sync App in production environments with high availability, monitoring, and security best practices.

## Overview

Sync App can be deployed in several configurations:
- **Single-node deployment**: Daemon + PocketBase on one server
- **Multi-node deployment**: Separate daemon instances with shared PocketBase
- **Container deployment**: Docker/Kubernetes orchestration
- **Cloud deployment**: AWS, GCP, Azure with managed services

## Prerequisites

- **System Requirements**: 
  - CPU: 2+ cores recommended
  - RAM: 4GB+ for moderate workloads
  - Disk: SSD recommended for PocketBase storage
  - Network: Stable internet connection for remote sync

- **Operating System**: 
  - Ubuntu 20.04+ (recommended)
  - CentOS 8+/RHEL 8+
  - Debian 11+
  - Windows Server 2019+

## Single-Node Deployment

### Using systemd (Linux)

1. **Create dedicated user**
   ```bash
   sudo useradd --system --shell /bin/false --home-dir /opt/sync-app sync-app
   sudo mkdir -p /opt/sync-app/{bin,config,data,logs}
   sudo chown -R sync-app:sync-app /opt/sync-app
   ```

2. **Install binaries**
   ```bash
   # Copy built binaries
   sudo cp target/release/{daemon,sync} /opt/sync-app/bin/
   sudo chmod +x /opt/sync-app/bin/*
   
   # Create symlinks for global access
   sudo ln -s /opt/sync-app/bin/daemon /usr/local/bin/sync-daemon
   sudo ln -s /opt/sync-app/bin/sync /usr/local/bin/sync-cli
   ```

3. **Create production configuration**
   ```bash
   sudo tee /opt/sync-app/config/daemon.toml > /dev/null << 'EOF'
   [pocketbase]
   url = "http://localhost:8090"
   admin_email = "admin@yourdomain.com"
   admin_password = "your-secure-password"
   timeout_secs = 30
   retry_attempts = 3
   
   [daemon]
   pid_file = "/opt/sync-app/data/daemon.pid"
   log_file = "/opt/sync-app/logs/daemon.log"
   log_level = "info"
   config_refresh_interval_secs = 300
   
   [telemetry]
   log_level = "info"
   json_logging = true
   console_logging = false
   log_file_path = "/opt/sync-app/logs/daemon.log"
   
   [telemetry.log_rotation]
   enabled = true
   frequency = "daily"
   keep_files = 30
   max_size_mb = 100
   
   [telemetry.metrics]
   enabled = true
   bind_address = "127.0.0.1"
   port = 9090
   
   [concurrency]
   max_concurrent_syncs = 8
   max_file_operations = 500
   sync_queue_size = 2000
   EOF
   ```

4. **Create systemd service for PocketBase**
   ```bash
   sudo tee /etc/systemd/system/pocketbase.service > /dev/null << 'EOF'
   [Unit]
   Description=PocketBase Backend
   After=network.target
   
   [Service]
   Type=simple
   User=sync-app
   Group=sync-app
   WorkingDirectory=/opt/sync-app/data
   ExecStart=/opt/sync-app/bin/pocketbase serve --http=127.0.0.1:8090
   Restart=always
   RestartSec=5
   StandardOutput=journal
   StandardError=journal
   SyslogIdentifier=pocketbase
   
   # Security settings
   NoNewPrivileges=yes
   PrivateTmp=yes
   ProtectSystem=strict
   ProtectHome=yes
   ReadWritePaths=/opt/sync-app/data
   
   [Install]
   WantedBy=multi-user.target
   EOF
   ```

5. **Create systemd service for daemon**
   ```bash
   sudo tee /etc/systemd/system/sync-daemon.service > /dev/null << 'EOF'
   [Unit]
   Description=Sync App Daemon
   After=network.target pocketbase.service
   Requires=pocketbase.service
   
   [Service]
   Type=simple
   User=sync-app
   Group=sync-app
   WorkingDirectory=/opt/sync-app
   ExecStart=/opt/sync-app/bin/daemon start --config /opt/sync-app/config/daemon.toml
   ExecReload=/bin/kill -HUP $MAINPID
   Restart=always
   RestartSec=5
   StandardOutput=journal
   StandardError=journal
   SyslogIdentifier=sync-daemon
   
   # Security settings
   NoNewPrivileges=yes
   PrivateTmp=yes
   ProtectSystem=strict
   ProtectHome=yes
   ReadWritePaths=/opt/sync-app
   
   [Install]
   WantedBy=multi-user.target
   EOF
   ```

6. **Start and enable services**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable pocketbase sync-daemon
   sudo systemctl start pocketbase
   
   # Wait for PocketBase to start
   sleep 5
   sudo systemctl start sync-daemon
   
   # Check status
   sudo systemctl status pocketbase sync-daemon
   ```

### Using Docker

1. **Create Docker Compose configuration**
   ```yaml
   # docker-compose.yml
   version: '3.8'
   
   services:
     pocketbase:
       image: ghcr.io/muchobien/pocketbase:latest
       container_name: sync-pocketbase
       restart: unless-stopped
       ports:
         - "8090:8090"
       volumes:
         - pocketbase_data:/pb_data
       healthcheck:
         test: ["CMD", "curl", "-f", "http://localhost:8090/api/health"]
         interval: 30s
         timeout: 10s
         retries: 3
   
     sync-daemon:
       build: .
       container_name: sync-daemon
       restart: unless-stopped
       depends_on:
         pocketbase:
           condition: service_healthy
       ports:
         - "9090:9090"  # Metrics endpoint
       volumes:
         - ./config:/config:ro
         - ./data:/data
         - ./logs:/logs
         - /path/to/sync/directories:/sync:rw
       environment:
         - RUST_LOG=info
         - CONFIG_PATH=/config/daemon.toml
   
   volumes:
     pocketbase_data:
   ```

2. **Create Dockerfile**
   ```dockerfile
   # Dockerfile
   FROM rust:1.75-slim as builder
   
   WORKDIR /app
   COPY . .
   RUN cargo build --release
   
   FROM debian:bookworm-slim
   
   RUN apt-get update && \
       apt-get install -y ca-certificates curl && \
       rm -rf /var/lib/apt/lists/*
   
   COPY --from=builder /app/target/release/daemon /usr/local/bin/
   COPY --from=builder /app/target/release/sync /usr/local/bin/
   
   RUN useradd --system --create-home sync-app
   USER sync-app
   
   EXPOSE 9090
   CMD ["daemon", "start", "--config", "/config/daemon.toml"]
   ```

3. **Deploy with Docker Compose**
   ```bash
   # Build and start services
   docker-compose up -d
   
   # Check logs
   docker-compose logs -f sync-daemon
   
   # Check health
   curl http://localhost:9090/metrics
   ```

## Multi-Node Deployment

For high availability and load distribution across multiple sync daemon instances.

### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Sync Node 1   │    │   Sync Node 2   │    │   Sync Node N   │
│   ┌─────────┐   │    │   ┌─────────┐   │    │   ┌─────────┐   │
│   │ Daemon  │   │    │   │ Daemon  │   │    │   │ Daemon  │   │
│   └─────────┘   │    │   └─────────┘   │    │   └─────────┘   │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   PocketBase    │
                    │   (Shared DB)   │
                    └─────────────────┘
```

### Load Balancer Configuration (Nginx)

```nginx
# /etc/nginx/sites-available/sync-app
upstream pocketbase {
    server 127.0.0.1:8090;
}

upstream sync-metrics {
    server 10.0.1.10:9090;  # Node 1
    server 10.0.1.11:9090;  # Node 2
    server 10.0.1.12:9090;  # Node 3
}

server {
    listen 80;
    server_name sync.yourdomain.com;
    
    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name sync.yourdomain.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    # PocketBase API
    location /api/ {
        proxy_pass http://pocketbase;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    # Metrics endpoint (protected)
    location /metrics {
        auth_basic "Metrics";
        auth_basic_user_file /etc/nginx/.htpasswd;
        proxy_pass http://sync-metrics;
    }
}
```

### Database Clustering (PocketBase)

For production, consider clustering PocketBase with external database:

```bash
# Using PostgreSQL as backend
docker run -d \
  --name pocketbase-cluster \
  -p 8090:8090 \
  -e PB_DATABASE_URL="postgres://user:pass@postgres-cluster:5432/pocketbase" \
  pocketbase:latest
```

## Kubernetes Deployment

### Namespace and ConfigMap

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: sync-app

---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: sync-daemon-config
  namespace: sync-app
data:
  daemon.toml: |
    [pocketbase]
    url = "http://pocketbase:8090"
    admin_email = "admin@yourdomain.com"
    admin_password = "secure-password"
    
    [daemon]
    log_level = "info"
    
    [telemetry.metrics]
    enabled = true
    bind_address = "0.0.0.0"
    port = 9090
```

### PocketBase Deployment

```yaml
# k8s/pocketbase.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pocketbase
  namespace: sync-app
spec:
  replicas: 1
  selector:
    matchLabels:
      app: pocketbase
  template:
    metadata:
      labels:
        app: pocketbase
    spec:
      containers:
      - name: pocketbase
        image: ghcr.io/muchobien/pocketbase:latest
        ports:
        - containerPort: 8090
        volumeMounts:
        - name: data
          mountPath: /pb_data
        livenessProbe:
          httpGet:
            path: /api/health
            port: 8090
          initialDelaySeconds: 30
          periodSeconds: 10
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: pocketbase-data

---
apiVersion: v1
kind: Service
metadata:
  name: pocketbase
  namespace: sync-app
spec:
  selector:
    app: pocketbase
  ports:
  - port: 8090
    targetPort: 8090

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: pocketbase-data
  namespace: sync-app
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

### Sync Daemon Deployment

```yaml
# k8s/daemon.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sync-daemon
  namespace: sync-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sync-daemon
  template:
    metadata:
      labels:
        app: sync-daemon
    spec:
      containers:
      - name: sync-daemon
        image: sync-app:latest
        ports:
        - containerPort: 9090
          name: metrics
        volumeMounts:
        - name: config
          mountPath: /config
        - name: data
          mountPath: /data
        env:
        - name: CONFIG_PATH
          value: "/config/daemon.toml"
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /metrics
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
      volumes:
      - name: config
        configMap:
          name: sync-daemon-config
      - name: data
        emptyDir: {}

---
apiVersion: v1
kind: Service
metadata:
  name: sync-daemon-metrics
  namespace: sync-app
  labels:
    app: sync-daemon
spec:
  selector:
    app: sync-daemon
  ports:
  - port: 9090
    targetPort: 9090
    name: metrics
```

## Security Configuration

### SSL/TLS Certificates

1. **Using Let's Encrypt**
   ```bash
   # Install Certbot
   sudo apt install certbot python3-certbot-nginx
   
   # Obtain certificate
   sudo certbot --nginx -d sync.yourdomain.com
   
   # Auto-renewal
   sudo crontab -e
   # Add: 0 12 * * * /usr/bin/certbot renew --quiet
   ```

2. **Using custom certificates**
   ```bash
   # Generate self-signed (development only)
   openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
   ```

### Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow from 10.0.0.0/8 to any port 8090  # Internal PocketBase
sudo ufw allow from 10.0.0.0/8 to any port 9090  # Internal metrics
sudo ufw --force enable

# Firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-rich-rule="rule family='ipv4' source address='10.0.0.0/8' port protocol='tcp' port='8090' accept"
sudo firewall-cmd --reload
```

### Authentication & Authorization

1. **PocketBase Admin Setup**
   ```bash
   # Set strong admin password
   curl -X POST http://localhost:8090/api/admins \
     -H "Content-Type: application/json" \
     -d '{
       "email": "admin@yourdomain.com",
       "password": "very-secure-password-here",
       "passwordConfirm": "very-secure-password-here"
     }'
   ```

2. **API Token Management**
   ```bash
   # Create service account for daemon
   curl -X POST http://localhost:8090/api/collections/users/records \
     -H "Content-Type: application/json" \
     -d '{
       "email": "daemon@yourdomain.com",
       "password": "daemon-service-password",
       "passwordConfirm": "daemon-service-password",
       "role": "service"
     }'
   ```

## Monitoring & Observability

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'sync-daemon'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
    metrics_path: /metrics

  - job_name: 'pocketbase'
    static_configs:
      - targets: ['localhost:8090']
    scrape_interval: 30s
    metrics_path: /api/health

rule_files:
  - "sync-app-alerts.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093
```

### Grafana Dashboards

Key metrics to monitor:
- Sync operation success rate
- File transfer throughput
- Error rates by sync job
- Daemon memory/CPU usage
- PocketBase response times
- Queue lengths

### Log Aggregation

```yaml
# filebeat.yml for ELK stack
filebeat.inputs:
- type: log
  paths:
    - /opt/sync-app/logs/*.log
  json.keys_under_root: true
  json.add_error_key: true
  fields:
    service: sync-app
    environment: production

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "sync-app-logs-%{+yyyy.MM.dd}"

processors:
  - add_host_metadata:
      when.not.contains.tags: forwarded
```

## Backup & Recovery

### Database Backup

```bash
#!/bin/bash
# backup-pocketbase.sh

BACKUP_DIR="/backup/pocketbase"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Stop PocketBase temporarily
sudo systemctl stop pocketbase

# Create backup
tar -czf "$BACKUP_DIR/pocketbase_backup_$DATE.tar.gz" /opt/sync-app/data/pb_data

# Restart PocketBase
sudo systemctl start pocketbase

# Clean old backups (keep 30 days)
find "$BACKUP_DIR" -name "pocketbase_backup_*.tar.gz" -mtime +30 -delete

echo "Backup completed: pocketbase_backup_$DATE.tar.gz"
```

### Configuration Backup

```bash
#!/bin/bash
# backup-configs.sh

BACKUP_DIR="/backup/sync-configs"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

# Export all sync configurations
/opt/sync-app/bin/sync export "$BACKUP_DIR/sync_configs_$DATE.json"

# Backup daemon configuration
cp /opt/sync-app/config/daemon.toml "$BACKUP_DIR/daemon_config_$DATE.toml"

echo "Configuration backup completed"
```

### Automated Backup Cron

```bash
# Add to crontab
sudo crontab -e

# Database backup daily at 2 AM
0 2 * * * /opt/sync-app/scripts/backup-pocketbase.sh

# Configuration backup weekly
0 3 * * 0 /opt/sync-app/scripts/backup-configs.sh
```

## Performance Tuning

### System Limits

```bash
# /etc/security/limits.conf
sync-app soft nofile 65536
sync-app hard nofile 65536
sync-app soft nproc 4096
sync-app hard nproc 4096

# /etc/systemd/system/sync-daemon.service.d/limits.conf
[Service]
LimitNOFILE=65536
LimitNPROC=4096
```

### Daemon Configuration Tuning

```toml
# High-performance configuration
[concurrency]
max_concurrent_syncs = 16        # 2x CPU cores
max_file_operations = 1000       # Higher for large file sets
sync_queue_size = 5000          # Larger queue for busy systems

[cache]
cache_dir = "/opt/sync-app/cache"
config_cache_ttl_secs = 60      # Faster config updates
file_metadata_cache_ttl_secs = 30  # Fresh metadata
enable_persistent_cache = true

[telemetry.log_rotation]
max_size_mb = 50               # Smaller files, more frequent rotation
keep_files = 14                # Reduce storage usage
```

## Troubleshooting Production Issues

### Common Issues

1. **High Memory Usage**
   ```bash
   # Check memory usage
   ps aux | grep daemon
   
   # Adjust configuration
   [concurrency]
   max_concurrent_syncs = 4  # Reduce if memory constrained
   ```

2. **Database Connection Issues**
   ```bash
   # Check PocketBase health
   curl http://localhost:8090/api/health
   
   # Check logs
   sudo journalctl -u pocketbase -f
   ```

3. **File Permission Errors**
   ```bash
   # Fix ownership
   sudo chown -R sync-app:sync-app /opt/sync-app
   
   # Check SELinux (if applicable)
   sudo setsebool -P httpd_can_network_connect 1
   ```

### Health Checks

```bash
#!/bin/bash
# health-check.sh

# Check daemon status
if ! systemctl is-active --quiet sync-daemon; then
    echo "ERROR: Sync daemon is not running"
    exit 1
fi

# Check PocketBase
if ! curl -f http://localhost:8090/api/health >/dev/null 2>&1; then
    echo "ERROR: PocketBase is not responding"
    exit 1
fi

# Check metrics endpoint
if ! curl -f http://localhost:9090/metrics >/dev/null 2>&1; then
    echo "ERROR: Metrics endpoint is not responding"
    exit 1
fi

echo "All services healthy"
```

This comprehensive deployment guide provides production-ready configurations for various environments. Remember to customize security settings, monitoring, and backup strategies according to your specific requirements and compliance needs.
