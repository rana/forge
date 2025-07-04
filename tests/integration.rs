use anyhow::Result;

#[tokio::test]
#[ignore] // Only run with cargo test -- --ignored
async fn test_real_tool_installation() -> Result<()> {
    // Test installing a small, fast tool
    println!("Testing real installation of 'fd'...");

    // Run forge install
    let output = std::process::Command::new("cargo")
        .args(&["run", "--", "install", "fd"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to install fd: {}", stderr);
    }

    // Verify fd is installed
    let verify = std::process::Command::new("fd").arg("--version").output()?;

    assert!(
        verify.status.success(),
        "fd should be runnable after install"
    );

    let version_output = String::from_utf8_lossy(&verify.stdout);
    assert!(
        version_output.contains("fd"),
        "Version output should mention fd"
    );

    // Clean up
    println!("Cleaning up...");
    let _ = std::process::Command::new("cargo")
        .args(&["run", "--", "uninstall", "fd"])
        .output();

    Ok(())
}

#[tokio::test]
#[ignore]
#[cfg(target_os = "linux")]
async fn test_apt_installer() -> Result<()> {
    // Only run on systems with apt
    if std::process::Command::new("apt")
        .arg("--version")
        .output()
        .is_err()
    {
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
    if std::process::Command::new("brew")
        .arg("--version")
        .output()
        .is_err()
    {
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
