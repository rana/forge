use anyhow::Result;
use forge::backend::{execute_install_with_runner, expand_template};
use forge::command::mock::MockCommandRunner;
use forge::knowledge::Knowledge;
use forge::platform::Platform;

#[tokio::test]
async fn test_cargo_installer_contract() -> Result<()> {
    let knowledge = Knowledge::load().await?;
    let installer = knowledge
        .installers
        .get("cargo")
        .expect("cargo installer should exist");

    let mock = MockCommandRunner::new();

    // Test ripgrep installation
    mock.expect(
        "cargo",
        &["install", "ripgrep", "--locked"],
        "Installed package `ripgrep v14.0.3`",
        true,
    );

    let tool_config = knowledge
        .tools
        .get("ripgrep")
        .and_then(|t| t.installers.get("cargo"))
        .expect("ripgrep should have cargo installer");

    let platform = Platform::detect()?;
    let result =
        execute_install_with_runner(installer, "ripgrep", tool_config, None, &platform, &mock)?;

    assert_eq!(result.version, "14.0.3");

    Ok(())
}

#[tokio::test]
async fn test_brew_installer_contract() -> Result<()> {
    let knowledge = Knowledge::load().await?;
    let installer = knowledge
        .installers
        .get("brew")
        .expect("brew installer should exist");

    let mock = MockCommandRunner::new();

    // Test bat installation
    mock.expect(
        "brew",
        &["install", "bat"],
        "Pouring bat--0.24.0.arm64_ventura.bottle.tar.gz",
        true,
    );

    let tool_config = knowledge
        .tools
        .get("bat")
        .and_then(|t| t.installers.get("brew"))
        .expect("bat should have brew installer");

    let platform = Platform::detect()?;
    let result =
        execute_install_with_runner(installer, "bat", tool_config, None, &platform, &mock)?;

    assert_eq!(result.version, "0.24.0");

    Ok(())
}

#[tokio::test]
async fn test_apt_installer_contract() -> Result<()> {
    let knowledge = Knowledge::load().await?;
    let installer = knowledge
        .installers
        .get("apt")
        .expect("apt installer should exist");

    let mock = MockCommandRunner::new();

    // Test bat installation via apt
    mock.expect(
        "sudo",
        &["apt", "install", "-y", "bat"],
        "Setting up bat (0.24.0-1) ...",
        true,
    );

    let tool_config = knowledge
        .tools
        .get("bat")
        .and_then(|t| t.installers.get("apt"))
        .expect("bat should have apt installer");

    let platform = Platform::detect()?;
    let result =
        execute_install_with_runner(installer, "bat", tool_config, None, &platform, &mock)?;

    assert_eq!(result.version, "0.24.0-1");

    Ok(())
}

#[tokio::test]
async fn test_installer_precedence() -> Result<()> {
    let knowledge = Knowledge::load().await?;

    let linux_precedence = knowledge
        .platforms
        .get("linux")
        .expect("linux platform should exist");

    assert_eq!(
        linux_precedence.precedence,
        vec!["script", "cargo", "github", "apt"]
    );

    // Test macOS precedence
    let macos_precedence = knowledge
        .platforms
        .get("macos")
        .expect("macos platform should exist");

    assert_eq!(
        macos_precedence.precedence,
        vec!["script", "cargo", "github", "brew"]
    );

    Ok(())
}

#[test]
fn test_template_expansion() {
    let platform = Platform {
        os: "linux".to_string(),
        arch: "x86_64".to_string(),
    };

    // Create a minimal tool installer config
    use forge::knowledge::ToolInstaller;
    let config = ToolInstaller {
        package: Some("ripgrep-custom".to_string()),
        repo: Some("BurntSushi/ripgrep".to_string()),
        pattern: Some("*linux*".to_string()),
        url: None,
        linux: None,
        macos: None,
        windows: None,
    };

    let template = "cargo install {package} for {os} on {arch}";
    let expanded = expand_template(template, "ripgrep", &config, None, &platform);

    assert_eq!(expanded, "cargo install ripgrep-custom for linux on x86_64");
}
