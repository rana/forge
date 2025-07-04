use anyhow::Result;

#[tokio::test]
#[ignore] // Only run with cargo test -- --ignored
async fn test_real_tool_installation() -> Result<()> {
    // Test installing a small, fast tool
    println!("Testing real installation of 'fd'...");
    
    // Run forge install with debug enabled
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "install", "fd"])
        .env("FORGE_DEBUG", "1")  // Enable debug output
        .output()?;
    
    // Always print output for debugging CI issues
    println!("\n=== FORGE INSTALL OUTPUT ===");
    println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
    println!("EXIT STATUS: {}", output.status);
    println!("=== END OUTPUT ===\n");
    
    if !output.status.success() {
        panic!("Failed to install fd");
    }
    
    // Verify fd is installed
    let verify = std::process::Command::new("fd")
        .arg("--version")
        .output()?;
    
    assert!(verify.status.success(), "fd should be runnable after install");
    
    let version_output = String::from_utf8_lossy(&verify.stdout);
    assert!(version_output.contains("fd"), "Version output should mention fd");
    
    // Clean up
    println!("Cleaning up...");
    let _ = std::process::Command::new("cargo")
        .args(&["run", "--", "uninstall", "fd"])
        .output();
    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn debug_cargo_install_output() -> Result<()> {
    println!("=== DEBUGGING CARGO INSTALL OUTPUT FORMAT ===");
    
    // Directly run cargo install to see its output format
    let output = std::process::Command::new("cargo")
        .args(&["install", "fd-find", "--locked"])
        .output()?;
    
    println!("Command: cargo install fd-find --locked");
    println!("\nSTDOUT ({} bytes):", output.stdout.len());
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("\nSTDERR ({} bytes):", output.stderr.len());
    println!("{}", String::from_utf8_lossy(&output.stderr));
    println!("\nExit status: {}", output.status);
    
    // Also test a simple regex match
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);
    
    println!("\n=== TESTING REGEX PATTERNS ===");
    
    // Test different patterns
    let patterns = [
        r"Installed package `fd-find v([0-9]+\.[0-9]+\.[0-9]+)` \(executable `fd`\)",
        r"Installed package `fd-find v([0-9]+\.[0-9]+\.[0-9]+[^`]*)`",
        r"fd-find v([0-9]+\.[0-9]+\.[0-9]+)",
        r"v([0-9]+\.[0-9]+\.[0-9]+)",
    ];
    
    for pattern in patterns {
        println!("\nTesting pattern: {}", pattern);
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(&combined) {
                println!("  ✓ Match found: {}", captures.get(0).unwrap().as_str());
                if let Some(version) = captures.get(1) {
                    println!("  ✓ Version captured: {}", version.as_str());
                }
            } else {
                println!("  ✗ No match");
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
#[ignore]
#[cfg(target_os = "linux")]
async fn test_apt_installer() -> Result<()> {
    // Only run on systems with apt
    if std::process::Command::new("apt").arg("--version").output().is_err() {
        println!("Skipping apt test - apt not available");
        return Ok(());
    }
    
    println!("Testing apt installer with 'bat'...");
    
    // Note: This test requires sudo, so it's mainly for CI
    // In CI, we'd set up passwordless sudo for apt
    
    Ok(())
}

#[tokio::test]
#[ignore]
#[cfg(target_os = "macos")]
async fn test_brew_installer() -> Result<()> {
    // Only run on systems with brew
    if std::process::Command::new("brew").arg("--version").output().is_err() {
        println!("Skipping brew test - brew not available");
        return Ok(());
    }
    
    println!("Testing brew installer with 'bat'...");
    
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "install", "bat", "-i", "brew"])
        .output()?;
    
    assert!(output.status.success(), "Should install bat via brew");
    
    // Clean up
    let _ = std::process::Command::new("cargo")
        .args(&["run", "--", "uninstall", "bat"])
        .output();
    
    Ok(())
}

#[tokio::test]
async fn test_forge_basics() -> Result<()> {
    // Test that forge can run and load knowledge
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "list"])
        .output()?;
    
    assert!(output.status.success(), "forge list should succeed");
    
    // Test help
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()?;
    
    assert!(output.status.success(), "forge help should succeed");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("forge"), "Help should mention forge");
    
    Ok(())
}
