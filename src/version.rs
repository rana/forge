use crate::knowledge::VersionCheck;
use anyhow::Result;
use serde_json::Value;
use std::process::Command;

fn normalize_version(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
}

pub async fn check_latest_version(
    _installer_name: &str,
    package: &str,
    version_check: Option<&VersionCheck>,
) -> Result<Option<String>> {
    // If no version check config, can't check
    let check = match version_check {
        Some(vc) => vc,
        None => return Ok(None),
    };

    match check.method.to_lowercase().as_str() {
        "command" => {
            // Run command to check version
            if let Some(cmd_template) = &check.command {
                let command: Vec<String> = cmd_template
                    .iter()
                    .map(|part| {
                        part.replace("{package}", package)
                            .replace("{repo}", package)
                    })
                    .collect();

                let output = Command::new(&command[0]).args(&command[1..]).output()?;

                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Special handling for apt-cache policy output
                    if command[0] == "apt-cache" && command.get(1) == Some(&"policy".to_string()) {
                        return Ok(extract_apt_installed_version(&stdout));
                    }

                    // Special handling for brew info JSON output
                    if command[0] == "brew" && command.get(1) == Some(&"info".to_string()) {
                        return Ok(extract_brew_version(&stdout));
                    }

                    // For other commands, return the first line trimmed and normalized
                    return Ok(stdout.lines().next().map(|s| normalize_version(s.trim())));
                }
            }
        }
        "api" => {
            // For API calls, we still need some extraction
            if let Some(url_template) = &check.url {
                let url = url_template
                    .replace("{package}", package)
                    .replace("{repo}", package);

                let output = Command::new("curl").args(["-s", &url]).output()?;

                if output.status.success() {
                    let response = String::from_utf8_lossy(&output.stdout);

                    // For crates.io API, extract the exact version
                    if url.contains("crates.io")
                        && check.path.as_deref() == Some("crate.max_version")
                    {
                        if let Ok(json) = serde_json::from_str::<Value>(&response) {
                            if let Some(version) =
                                json.pointer("/crate/max_version").and_then(|v| v.as_str())
                            {
                                return Ok(Some(normalize_version(version)));
                            }
                        }
                    }

                    // For GitHub API, extract tag name
                    if url.contains("api.github.com") && check.path.as_deref() == Some("tag_name") {
                        if let Ok(json) = serde_json::from_str::<Value>(&response) {
                            if let Some(version) = json.get("tag_name").and_then(|v| v.as_str()) {
                                return Ok(Some(normalize_version(version)));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(None)
}

fn extract_apt_installed_version(output: &str) -> Option<String> {
    // Look for "Installed: <version>" line
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Installed:") {
            let version = line.trim_start_matches("Installed:").trim();
            // Skip if "(none)" or empty
            if version != "(none)" && !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}

fn extract_brew_version(json_output: &str) -> Option<String> {
    // Parse brew's JSON output to get installed version
    if let Ok(json) = serde_json::from_str::<Value>(json_output) {
        // brew info returns { "formulae": [...], "casks": [...] }
        if let Some(formulae) = json.get("formulae").and_then(|f| f.as_array()) {
            if let Some(formula) = formulae.first() {
                if let Some(installed) = formula.get("installed").and_then(|i| i.as_array()) {
                    if let Some(version_info) = installed.first() {
                        if let Some(version) = version_info.get("version").and_then(|v| v.as_str())
                        {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
