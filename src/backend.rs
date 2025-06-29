use crate::knowledge::{Installer, Knowledge, ToolInstaller};
use crate::platform::Platform;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;

pub struct InstallResult {
    pub version: String,
}

pub fn execute_install(
    installer: &Installer,
    tool_name: &str,
    tool_config: &ToolInstaller,
    version: Option<&str>,
    platform: &Platform,
    knowledge: &Knowledge,
) -> Result<InstallResult> {
    let mut command = installer.install.clone();

    // Expand templates
    for part in &mut command {
        *part = expand_template(part, tool_name, tool_config, version, platform);
    }

    println!("🔨 Running: {}", command.join(" "));

    let output = Command::new(&command[0]).args(&command[1..]).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }

    // Extract version from output - REQUIRED
    let pattern_template = installer
        .install_output_pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No install_output_pattern defined for this installer"))?;

    // Expand both template variables and pattern references
    let mut pattern = expand_template(pattern_template, tool_name, tool_config, version, platform);
    pattern = expand_pattern_refs(&pattern, &knowledge.patterns);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check both stdout and stderr
    let version = extract_with_pattern(&stdout, &pattern)
        .or_else(|| extract_with_pattern(&stderr, &pattern))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to extract version from install output.\nPattern: {}\nHint: Run with FORGE_DEBUG=1 to see full output", 
                pattern
            )
        })?;

    Ok(InstallResult { version })
}

pub fn execute_script_install(
    script: &str,
    _tool_name: &str,
    platform: &Platform,
) -> Result<InstallResult> {
    let expanded_script = platform.expand_pattern(script);

    println!("🔍 This will run the following script:");
    println!("{}", crate::color::Colors::muted(&expanded_script));
    println!("\n⚠️  Please review the script before proceeding.");
    print!("Continue? [y/N] ");

    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        anyhow::bail!("Installation cancelled");
    }

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

fn expand_pattern_refs(pattern: &str, patterns: &HashMap<String, String>) -> String {
    let mut result = pattern.to_string();

    // Find all ${pattern_name} references and replace them
    for (name, value) in patterns {
        let placeholder = format!("${{{}}}", name);
        result = result.replace(&placeholder, value);
    }

    result
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
