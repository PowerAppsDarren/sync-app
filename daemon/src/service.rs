use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// Platform-specific implementations are included inline

pub async fn install_service(
    service_name: &str,
    description: &str,
    config_path: Option<&PathBuf>,
) -> Result<()> {
    info!("Installing service: {}", service_name);
    
    #[cfg(windows)]
    {
        windows::install_windows_service(service_name, description, config_path).await
    }
    
    #[cfg(target_os = "macos")]
    {
        macos::install_launchd_service(service_name, description, config_path).await
    }
    
    #[cfg(target_os = "linux")]
    {
        linux::install_systemd_service(service_name, description, config_path).await
    }
    
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Err(anyhow::anyhow!("Service installation not supported on this platform"))
    }
}

pub async fn uninstall_service(service_name: &str) -> Result<()> {
    info!("Uninstalling service: {}", service_name);
    
    #[cfg(windows)]
    {
        windows::uninstall_windows_service(service_name).await
    }
    
    #[cfg(target_os = "macos")]
    {
        macos::uninstall_launchd_service(service_name).await
    }
    
    #[cfg(target_os = "linux")]
    {
        linux::uninstall_systemd_service(service_name).await
    }
    
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Err(anyhow::anyhow!("Service uninstallation not supported on this platform"))
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::OsString;
    use std::process::Command;
    
    pub async fn install_windows_service(
        service_name: &str,
        description: &str,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        // Get current executable path
        let exe_path = std::env::current_exe()?;
        
        // Build service command
        let mut service_cmd = format!(
            "\"{}\" start --foreground",
            exe_path.display()
        );
        
        if let Some(config) = config_path {
            service_cmd.push_str(&format!(" --config \"{}\"", config.display()));
        }
        
        // Generate NSSM command for service installation
        let nssm_install = generate_nssm_install_commands(
            service_name,
            description,
            &exe_path,
            config_path,
        );
        
        // Try to install with sc command first (built-in Windows service manager)
        let sc_result = install_with_sc_command(service_name, description, &service_cmd);
        
        match sc_result {
            Ok(_) => {
                println!("✓ Service installed successfully using Windows Service Manager");
                println!("  Service Name: {}", service_name);
                println!("  Description: {}", description);
                println!("  Executable: {}", exe_path.display());
                
                // Start the service
                start_windows_service(service_name).await?;
                Ok(())
            }
            Err(e) => {
                warn!("Failed to install with sc command: {}", e);
                println!("⚠ Windows Service Manager installation failed");
                println!("📋 Alternative: Use NSSM (Non-Sucking Service Manager)");
                println!();
                println!("To install with NSSM, run the following commands as Administrator:");
                for cmd in nssm_install {
                    println!("  {}", cmd);
                }
                println!();
                println!("Download NSSM from: https://nssm.cc/download");
                Ok(())
            }
        }
    }
    
    pub async fn uninstall_windows_service(service_name: &str) -> Result<()> {
        // Stop the service first
        let _ = stop_windows_service(service_name).await;
        
        // Remove the service
        let output = Command::new("sc")
            .args(&["delete", service_name])
            .output()?;
        
        if output.status.success() {
            println!("✓ Service '{}' uninstalled successfully", service_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            if error.contains("does not exist") {
                println!("ℹ Service '{}' was not installed", service_name);
            } else {
                return Err(anyhow::anyhow!("Failed to uninstall service: {}", error));
            }
        }
        
        Ok(())
    }
    
    fn install_with_sc_command(
        service_name: &str,
        description: &str,
        service_cmd: &str,
    ) -> Result<()> {
        // Create the service
        let output = Command::new("sc")
            .args(&[
                "create",
                service_name,
                "binPath=",
                service_cmd,
                "start=",
                "auto",
                "DisplayName=",
                description,
            ])
            .output()?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to create service: {}", error));
        }
        
        // Set the description
        let _ = Command::new("sc")
            .args(&["description", service_name, description])
            .output();
        
        Ok(())
    }
    
    async fn start_windows_service(service_name: &str) -> Result<()> {
        let output = Command::new("sc")
            .args(&["start", service_name])
            .output()?;
        
        if output.status.success() {
            println!("✓ Service '{}' started successfully", service_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to start service: {}", error);
        }
        
        Ok(())
    }
    
    async fn stop_windows_service(service_name: &str) -> Result<()> {
        let output = Command::new("sc")
            .args(&["stop", service_name])
            .output()?;
        
        if output.status.success() {
            println!("✓ Service '{}' stopped successfully", service_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            if !error.contains("not started") {
                warn!("Failed to stop service: {}", error);
            }
        }
        
        Ok(())
    }
    
    fn generate_nssm_install_commands(
        service_name: &str,
        description: &str,
        exe_path: &Path,
        config_path: Option<&PathBuf>,
    ) -> Vec<String> {
        let mut commands = vec![
            format!("nssm install {} \"{}\"", service_name, exe_path.display()),
            format!("nssm set {} Application \"{}\"", service_name, exe_path.display()),
            format!("nssm set {} Parameters \"start --foreground{}\"", 
                service_name,
                if let Some(config) = config_path {
                    format!(" --config \\\"{}\\\"", config.display())
                } else {
                    String::new()
                }
            ),
            format!("nssm set {} DisplayName \"{}\"", service_name, description),
            format!("nssm set {} Description \"{}\"", service_name, description),
            format!("nssm set {} Start SERVICE_AUTO_START", service_name),
            format!("nssm set {} AppStdout \"C:\\ProgramData\\{}\\stdout.log\"", service_name, service_name),
            format!("nssm set {} AppStderr \"C:\\ProgramData\\{}\\stderr.log\"", service_name, service_name),
            format!("nssm start {}", service_name),
        ];
        
        commands
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::fs;
    use std::process::Command;
    
    pub async fn install_launchd_service(
        service_name: &str,
        description: &str,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        let plist_content = generate_launchd_plist(service_name, description, config_path)?;
        let plist_path = get_launchd_plist_path(service_name);
        
        // Create directory if it doesn't exist
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write the plist file
        fs::write(&plist_path, plist_content)?;
        
        // Load the service
        let output = Command::new("launchctl")
            .args(&["load", "-w", plist_path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            println!("✓ Service installed successfully using launchd");
            println!("  Service Name: {}", service_name);
            println!("  Plist Path: {}", plist_path.display());
            println!("  Description: {}", description);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to load service: {}", error));
        }
        
        Ok(())
    }
    
    pub async fn uninstall_launchd_service(service_name: &str) -> Result<()> {
        let plist_path = get_launchd_plist_path(service_name);
        
        // Unload the service
        let output = Command::new("launchctl")
            .args(&["unload", "-w", plist_path.to_str().unwrap()])
            .output()?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            if !error.contains("not found") {
                warn!("Failed to unload service: {}", error);
            }
        }
        
        // Remove the plist file
        if plist_path.exists() {
            fs::remove_file(&plist_path)?;
            println!("✓ Service '{}' uninstalled successfully", service_name);
        } else {
            println!("ℹ Service '{}' was not installed", service_name);
        }
        
        Ok(())
    }
    
    fn get_launchd_plist_path(service_name: &str) -> PathBuf {
        PathBuf::from(format!("/Library/LaunchDaemons/com.syncapp.{}.plist", service_name))
    }
    
    fn generate_launchd_plist(
        service_name: &str,
        description: &str,
        config_path: Option<&PathBuf>,
    ) -> Result<String> {
        let exe_path = std::env::current_exe()?;
        
        let mut program_arguments = vec![
            format!("\"{}\"", exe_path.display()),
            "start".to_string(),
            "--foreground".to_string(),
        ];
        
        if let Some(config) = config_path {
            program_arguments.push("--config".to_string());
            program_arguments.push(format!("\"{}\"", config.display()));
        }
        
        let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.syncapp.{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>start</string>
        <string>--foreground</string>{}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>WorkingDirectory</key>
    <string>/var/log</string>
    <key>StandardOutPath</key>
    <string>/var/log/{}.out</string>
    <key>StandardErrorPath</key>
    <string>/var/log/{}.err</string>
    <key>UserName</key>
    <string>root</string>
</dict>
</plist>"#,
            service_name,
            exe_path.display(),
            if let Some(config) = config_path {
                format!("\n        <string>--config</string>\n        <string>{}</string>", config.display())
            } else {
                String::new()
            },
            service_name,
            service_name
        );
        
        Ok(plist)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::fs;
    use std::process::Command;
    
    pub async fn install_systemd_service(
        service_name: &str,
        description: &str,
        config_path: Option<&PathBuf>,
    ) -> Result<()> {
        let unit_content = generate_systemd_unit(service_name, description, config_path)?;
        let unit_path = get_systemd_unit_path(service_name);
        
        // Write the unit file
        fs::write(&unit_path, unit_content)?;
        
        // Reload systemd
        let output = Command::new("systemctl")
            .args(&["daemon-reload"])
            .output()?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to reload systemd: {}", error));
        }
        
        // Enable the service
        let output = Command::new("systemctl")
            .args(&["enable", service_name])
            .output()?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to enable service: {}", error));
        }
        
        // Start the service
        let output = Command::new("systemctl")
            .args(&["start", service_name])
            .output()?;
        
        if output.status.success() {
            println!("✓ Service installed and started successfully using systemd");
            println!("  Service Name: {}", service_name);
            println!("  Unit File: {}", unit_path.display());
            println!("  Description: {}", description);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("Service installed but failed to start: {}", error);
        }
        
        Ok(())
    }
    
    pub async fn uninstall_systemd_service(service_name: &str) -> Result<()> {
        // Stop the service
        let _ = Command::new("systemctl")
            .args(&["stop", service_name])
            .output();
        
        // Disable the service
        let _ = Command::new("systemctl")
            .args(&["disable", service_name])
            .output();
        
        // Remove the unit file
        let unit_path = get_systemd_unit_path(service_name);
        if unit_path.exists() {
            fs::remove_file(&unit_path)?;
        }
        
        // Reload systemd
        let output = Command::new("systemctl")
            .args(&["daemon-reload"])
            .output()?;
        
        if output.status.success() {
            println!("✓ Service '{}' uninstalled successfully", service_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to reload systemd after uninstall: {}", error);
        }
        
        Ok(())
    }
    
    fn get_systemd_unit_path(service_name: &str) -> PathBuf {
        PathBuf::from(format!("/etc/systemd/system/{}.service", service_name))
    }
    
    fn generate_systemd_unit(
        service_name: &str,
        description: &str,
        config_path: Option<&PathBuf>,
    ) -> Result<String> {
        let exe_path = std::env::current_exe()?;
        
        let exec_start = if let Some(config) = config_path {
            format!("{} start --foreground --config \"{}\"", exe_path.display(), config.display())
        } else {
            format!("{} start --foreground", exe_path.display())
        };
        
        let unit = format!(r#"[Unit]
Description={}
After=network.target
Wants=network.target

[Service]
Type=simple
ExecStart={}
Restart=always
RestartSec=5
User=root
Group=root
StandardOutput=journal
StandardError=journal
SyslogIdentifier={}

[Install]
WantedBy=multi-user.target"#,
            description,
            exec_start,
            service_name
        );
        
        Ok(unit)
    }
}
