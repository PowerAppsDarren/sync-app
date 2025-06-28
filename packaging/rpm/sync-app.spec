Name:           sync-app
Version:        0.1.0
Release:        1%{?dist}
Summary:        Cross-platform synchronization application with PocketBase backend

Group:          Applications/Internet
License:        AGPL-3.0
URL:            https://github.com/yourusername/sync-app
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  systemd-rpm-macros
Requires:       glibc, openssl-libs
Recommends:     sqlite
Suggests:       nginx, httpd

Requires(pre):  shadow-utils
Requires(post): systemd
Requires(preun): systemd
Requires(postun): systemd

%description
Sync App is a modern, cross-platform synchronization application built with
Rust and powered by PocketBase. It provides multiple interfaces including
a command-line tool, server component, and background daemon.

Features:
- Cross-platform support (Linux, macOS, Windows)
- Real-time synchronization across devices  
- PocketBase integration for backend database and API
- Secure communication with built-in encryption
- High performance with Rust implementation

%prep
%setup -q

%build
# Binaries are pre-built in the source archive

%install
rm -rf $RPM_BUILD_ROOT

# Create directory structure
install -d $RPM_BUILD_ROOT%{_bindir}
install -d $RPM_BUILD_ROOT%{_sysconfdir}/%{name}
install -d $RPM_BUILD_ROOT%{_unitdir}
install -d $RPM_BUILD_ROOT%{_localstatedir}/lib/%{name}
install -d $RPM_BUILD_ROOT%{_localstatedir}/log/%{name}
install -d $RPM_BUILD_ROOT%{_docdir}/%{name}
install -d $RPM_BUILD_ROOT%{_mandir}/man1

# Install binaries
install -m 755 sync $RPM_BUILD_ROOT%{_bindir}/sync
install -m 755 sync-server $RPM_BUILD_ROOT%{_bindir}/sync-server
install -m 755 sync-daemon $RPM_BUILD_ROOT%{_bindir}/sync-daemon
install -m 755 pocketbase $RPM_BUILD_ROOT%{_bindir}/pocketbase

# Install configuration
cat > $RPM_BUILD_ROOT%{_sysconfdir}/%{name}/config.yaml << EOF
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

# Install systemd service
cat > $RPM_BUILD_ROOT%{_unitdir}/sync-daemon.service << EOF
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

# Install documentation
install -m 644 README.md $RPM_BUILD_ROOT%{_docdir}/%{name}/README.md 2>/dev/null || echo "README.md not found, creating basic one"
cat > $RPM_BUILD_ROOT%{_docdir}/%{name}/README.md << EOF
# Sync App

Cross-platform synchronization application with PocketBase backend.

## Components

- \`sync\`: Command-line interface
- \`sync-server\`: Server component
- \`sync-daemon\`: Background daemon service
- \`pocketbase\`: PocketBase database (optional)

## Configuration

System configuration: /etc/sync-app/config.yaml
User data: /var/lib/sync-app/
Logs: /var/log/sync-app/

## Service Management

The sync-daemon is installed as a systemd service:

    sudo systemctl start sync-daemon    # Start the service
    sudo systemctl stop sync-daemon     # Stop the service
    sudo systemctl enable sync-daemon   # Enable auto-start
    sudo systemctl disable sync-daemon  # Disable auto-start
    sudo systemctl status sync-daemon   # Check status

For more information, visit: https://github.com/yourusername/sync-app
EOF

%files
%doc %{_docdir}/%{name}/README.md
%config(noreplace) %{_sysconfdir}/%{name}/config.yaml
%{_bindir}/sync
%{_bindir}/sync-server
%{_bindir}/sync-daemon
%{_bindir}/pocketbase
%{_unitdir}/sync-daemon.service

# Directories that will be created with proper ownership
%attr(755, sync-app, sync-app) %dir %{_localstatedir}/lib/%{name}
%attr(755, sync-app, sync-app) %dir %{_localstatedir}/log/%{name}

%pre
# Create sync-app user and group
getent group sync-app >/dev/null || groupadd -r sync-app
getent passwd sync-app >/dev/null || \
    useradd -r -g sync-app -d /var/lib/sync-app -s /sbin/nologin \
    -c "Sync App daemon" sync-app
exit 0

%post
%systemd_post sync-daemon.service

# Set proper ownership for directories
chown -R sync-app:sync-app /var/lib/sync-app
chown -R sync-app:sync-app /var/log/sync-app

echo "Sync App installed successfully!"
echo "Configuration: /etc/sync-app/config.yaml"
echo "Commands: sync, sync-server, sync-daemon, pocketbase"
echo "Enable and start service: systemctl enable --now sync-daemon"

%preun
%systemd_preun sync-daemon.service

%postun
%systemd_postun_with_restart sync-daemon.service

# Remove user and group on complete removal
if [ $1 -eq 0 ] ; then
    getent passwd sync-app >/dev/null && userdel sync-app 2>/dev/null || true
    getent group sync-app >/dev/null && groupdel sync-app 2>/dev/null || true
    rm -rf /var/lib/sync-app
    rm -rf /var/log/sync-app
fi

%changelog
* %{date} Your Name <your.email@example.com> - 0.1.0-1
- Initial RPM package release
- Cross-platform synchronization with PocketBase backend
- CLI tool, server component, and daemon service
- Systemd service integration
