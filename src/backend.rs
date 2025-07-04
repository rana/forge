use crate::command::{CommandRunner, SystemCommandRunner};
use crate::knowledge::{Installer, Tool, ToolInstaller};
use crate::platform::Platform;
use anyhow::Result;
use regex::Regex;
use std::process::Command;

pub struct InstallResult {
    pub version: String,
    pub executables: Option<Vec<String>>,
}

pub fn execute_install(
    installer: &Installer,
    tool_name: &str,
    tool_config: &ToolInstaller,
    version: Option<&str>,
    platform: &Platform,
) -> Result<InstallResult> {
    execute_install_with_runner(
        installer,
        tool_name,
        tool_config,
        version,
        platform,
        &SystemCommandRunner,
    )
}

pub fn execute_install_with_runner(
    installer: &Installer,
    tool_name: &str,
    tool_config: &ToolInstaller,
    version: Option<&str>,
    platform: &Platform,
    runner: &dyn CommandRunner,
) -> Result<InstallResult> {
    let mut command = installer.install.clone();

    // Expand templates
    for part in &mut command {
        *part = expand_template(part, tool_name, tool_config, version, platform);
    }

    println!("🔨 Running: {}", command.join(" "));

    let output = runner.run(&command[0], &command[1..])?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }

    // Extract version from output - REQUIRED
    let pattern_template = installer
        .install_output_pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No install_output_pattern defined for this installer"))?;

    // Just expand template variables, no pattern refs
    let pattern = expand_template(pattern_template, tool_name, tool_config, version, platform);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // Check combined output
    let version = extract_with_pattern(&combined, &pattern)
        .ok_or_else(|| {
            if std::env::var("FORGE_DEBUG").is_ok() {
                eprintln!("DEBUG: Pattern: {}", pattern);
                eprintln!("DEBUG: Output:\n{}", combined);
            }
            anyhow::anyhow!(
                "Failed to extract version from install output.\nPattern: {}\nHint: Run with FORGE_DEBUG=1 to see full output", 
                pattern
            )
        })?;

    Ok(InstallResult {
        version,
        executables: None,
    })
}

pub fn execute_script_install(
    script: &str,
    _tool_name: &str,
    platform: &Platform,
) -> Result<InstallResult> {
    let expanded_script = platform.expand_pattern(script);

    println!("🔍 Running the following script:");
    println!("{}", crate::color::Colors::muted(&expanded_script));

    println!("🔨 Running installer script...");

    // Execute via sh -c
    let output = Command::new("sh")
        .arg("-c")
        .arg(&expanded_script)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Script failed: {}", stderr);
    }

    // For scripts, we can't reliably extract version
    // The tool will be detected on next run
    Ok(InstallResult {
        version: "installed".to_string(),
        executables: None,
    })
}

pub fn execute_github_install(
    tool_name: &str,
    tool_config: &ToolInstaller,
    tool: &Tool,
    platform: &Platform,
) -> Result<InstallResult> {
    use crate::github::{discover_asset, download_and_install};

    let repo = tool_config
        .repo
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("GitHub installer requires 'repo' field"))?;

    // If pattern is provided, use the old behavior
    if let Some(pattern) = &tool_config.pattern {
        // Use existing gh CLI approach
        let expanded_pattern = platform.expand_pattern(pattern);

        let output = Command::new("gh")
            .args(&[
                "release",
                "download",
                "--repo",
                repo,
                "--pattern",
                &expanded_pattern,
                "--skip-existing",
                "--dir",
                "~/.local/bin",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("GitHub download failed: {}", stderr);
        }

        // Extract version from output if possible
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = extract_version(&stdout).unwrap_or_else(|| "unknown".to_string());

        return Ok(InstallResult {
            version,
            executables: None
        });
    }

    // Smart discovery path
    let discovery = discover_asset(repo, &platform.os, &platform.arch)?;

    // Get provides hint from tool definition
    let provides_hint = &tool.provides;

    // Download and install
    let install_result = download_and_install(
        &discovery.download_url,
        &discovery.asset_name,
        tool_name,
        provides_hint,
    )?;

    // Print what we installed
    for exe in &install_result.executables {
        println!("  Installed: {}", exe);
    }

    Ok(InstallResult {
        version: discovery.version,
        executables: Some(install_result.executables),
    })
}

pub fn expand_template(
    template: &str,
    tool_name: &str,
    config: &ToolInstaller,
    version: Option<&str>,
    platform: &Platform,
) -> String {
    let expanded = template
        .replace("{tool}", tool_name)
        .replace("{package}", config.package.as_deref().unwrap_or(tool_name))
        .replace("{repo}", config.repo.as_deref().unwrap_or(""))
        .replace("{pattern}", config.pattern.as_deref().unwrap_or("*"))
        .replace("{url}", config.url.as_deref().unwrap_or(""))
        .replace("{version}", version.unwrap_or("latest"));

    platform.expand_pattern(&expanded)
}

pub fn check_tool_version(tool_name: &str, command_template: &[String]) -> Result<Option<String>> {
    let command: Vec<String> = command_template
        .iter()
        .map(|part| part.replace("{tool}", tool_name))
        .collect();

    if command.is_empty() {
        return Ok(None);
    }

    let output = Command::new(&command[0]).args(&command[1..]).output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(extract_version(&stdout))
    } else {
        Ok(None)
    }
}

fn extract_version(output: &str) -> Option<String> {
    use regex::Regex;
    let re = Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
    re.captures(output)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_with_pattern(text: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()
        .and_then(|re| re.captures(text))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}
