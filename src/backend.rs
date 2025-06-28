use crate::knowledge::{Installer, ToolInstaller};
use anyhow::Result;
use std::process::Command;
use regex::Regex;

pub struct InstallResult {
    pub version: String,
}

pub fn execute_install(
    installer: &Installer,
    tool_name: &str,
    tool_config: &ToolInstaller,
    version: Option<&str>,
) -> Result<InstallResult> {
    let mut command = installer.install.clone();
    
    // Expand templates
    for part in &mut command {
        *part = expand_template(part, tool_name, tool_config, version);
    }
    
    println!("🔨 Running: {}", command.join(" "));
    
    let output = Command::new(&command[0])
        .args(&command[1..])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }
    
    // Try to extract version from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = extract_version(&stdout).unwrap_or_else(|| "unknown".to_string());
    
    Ok(InstallResult { version })
}

fn expand_template(
    template: &str,
    tool_name: &str,
    config: &ToolInstaller,
    version: Option<&str>,
) -> String {
    template
        .replace("{tool}", tool_name)
        .replace("{package}", config.package.as_deref().unwrap_or(tool_name))
        .replace("{repo}", config.repo.as_deref().unwrap_or(""))
        .replace("{pattern}", config.pattern.as_deref().unwrap_or("*"))
        .replace("{url}", config.url.as_deref().unwrap_or(""))
        .replace("{version}", version.unwrap_or("latest"))
}

fn extract_version(output: &str) -> Option<String> {
    let re = Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
    re.captures(output)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn check_tool_version(tool_name: &str, command_template: &[String]) -> Result<Option<String>> {
    let command: Vec<String> = command_template
        .iter()
        .map(|part| part.replace("{tool}", tool_name))
        .collect();
    
    if command.is_empty() {
        return Ok(None);
    }
    
    let output = Command::new(&command[0])
        .args(&command[1..])
        .output()?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(extract_version(&stdout))
    } else {
        Ok(None)
    }
}
