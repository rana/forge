use crate::knowledge::VersionCheck;
use anyhow::Result;
use regex::Regex;
use std::process::Command;

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
                    .map(|part| part.replace("{package}", package))
                    .collect();

                let output = Command::new(&command[0]).args(&command[1..]).output()?;

                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    return Ok(extract_version_from_text(&stdout));
                }
            }
        }
        "api" => {
            // For now, implement a simple HTTP check
            if let Some(url_template) = &check.url {
                let url = url_template.replace("{package}", package);

                // Use simple command-based approach for now
                let output = Command::new("curl").args(["-s", &url]).output()?;

                if output.status.success() {
                    let response = String::from_utf8_lossy(&output.stdout);

                    // Extract version based on path
                    if let Some(path) = &check.path {
                        // Simple JSON extraction - look for pattern like "max_version":"1.2.3"
                        let pattern = format!(
                            r#""{}"\s*:\s*"([^"]+)""#,
                            path.split('.').last().unwrap_or(path)
                        );
                        if let Ok(re) = Regex::new(&pattern) {
                            if let Some(captures) = re.captures(&response) {
                                if let Some(version) = captures.get(1) {
                                    return Ok(Some(version.as_str().to_string()));
                                }
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

fn extract_version_from_text(text: &str) -> Option<String> {
    // Try common version patterns
    let patterns = [
        r"(\d+\.\d+\.\d+)",
        r"v(\d+\.\d+\.\d+)",
        r"version[:\s]+(\d+\.\d+\.\d+)",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(text) {
                if let Some(version) = captures.get(1) {
                    return Some(version.as_str().to_string());
                }
            }
        }
    }

    None
}
