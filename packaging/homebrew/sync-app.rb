class SyncApp < Formula
  desc "Cross-platform synchronization application with PocketBase backend"
  homepage "https://github.com/yourusername/sync-app"
  version "0.1.0"
  license "AGPL-3.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yourusername/sync-app/releases/download/v0.1.0/sync-app-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_ARM64_DARWIN_PLACEHOLDER"
    else
      url "https://github.com/yourusername/sync-app/releases/download/v0.1.0/sync-app-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_X86_64_DARWIN_PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/yourusername/sync-app/releases/download/v0.1.0/sync-app-0.1.0-aarch64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_ARM64_LINUX_PLACEHOLDER"
    else
      url "https://github.com/yourusername/sync-app/releases/download/v0.1.0/sync-app-0.1.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_X86_64_LINUX_PLACEHOLDER"
    end
  end

  depends_on "sqlite" => :optional

  def install
    # Install main binaries
    bin.install "sync"
    bin.install "sync-server" 
    bin.install "sync-daemon"
    
    # Install PocketBase as optional dependency
    bin.install "pocketbase"
    
    # Install shell completions
    generate_completions_from_executable(bin/"sync", "completion")
    
    # Install man pages if they exist
    if File.exist?("man")
      man1.install Dir["man/*.1"]
    end
    
    # Install configuration examples
    pkgshare.install "examples" if File.exist?("examples")
    
    # Install service files for launchd (macOS)
    if OS.mac?
      (prefix/"LaunchDaemons").install "packaging/macos/com.sync-app.daemon.plist"
    end
  end

  def post_install
    # Create config directory
    (var/"lib/sync-app").mkpath
    (etc/"sync-app").mkpath
    
    # Copy default config if it doesn't exist
    unless (etc/"sync-app/config.yaml").exist?
      cp pkgshare/"examples/config.yaml", etc/"sync-app/config.yaml"
    end
  end

  service do
    run [opt_bin/"sync-daemon", "--config", etc/"sync-app/config.yaml"]
    working_dir var/"lib/sync-app"
    log_path var/"log/sync-app.log"
    error_log_path var/"log/sync-app-error.log"
    environment_variables PATH: std_service_path_env
    keep_alive true
    require_root false
  end

  test do
    # Test binary execution
    assert_match version.to_s, shell_output("#{bin}/sync --version")
    assert_match version.to_s, shell_output("#{bin}/sync-server --version")
    assert_match version.to_s, shell_output("#{bin}/sync-daemon --version")
    
    # Test PocketBase is included
    assert_predicate bin/"pocketbase", :exist?
    
    # Test config creation
    (testpath/"test-config.yaml").write <<~EOS
      server:
        host: "127.0.0.1"
        port: 8080
      database:
        path: "./test.db"
      logging:
        level: "info"
    EOS
    
    # Test daemon can start and stop (without actually running)
    system bin/"sync-daemon", "--config", testpath/"test-config.yaml", "--check-config"
  end
end
