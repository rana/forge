use crate::color::{Colors, ERROR};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    pub gist_id: String,
    pub gist_url: String,
    pub last_hash: String,
    pub last_sync: DateTime<Utc>,
}

/// Check if gh CLI is available and authenticated
pub fn check_gh_auth() -> Result<()> {
    // Check if gh exists
    let gh_check = Command::new("gh").arg("--version").output();

    if gh_check.is_err() || !gh_check.unwrap().status.success() {
        anyhow::bail!(
            "{} GitHub CLI (gh) not found\n\
            {} Install with: {}",
            ERROR,
            crate::color::TIP,
            Colors::action("forge install gh")
        );
    }

    // Check if authenticated
    let auth_check = Command::new("gh").args(&["auth", "status"]).output()?;

    if !auth_check.status.success() {
        anyhow::bail!(
            "{} GitHub CLI not authenticated\n\
            {} Authenticate with: {}",
            ERROR,
            crate::color::TIP,
            Colors::action("gh auth login")
        );
    }

    Ok(())
}

/// Get current GitHub username
pub fn get_github_user() -> Result<String> {
    let output = Command::new("gh")
        .args(&["api", "user", "--jq", ".login"])
        .output()
        .context("Failed to get GitHub user")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get GitHub username");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Calculate SHA256 hash of file contents
pub fn hash_file_contents(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn extract_gist_id(url: &str) -> Result<String> {
    // Parse the URL to extract gist ID
    // Formats:
    // - https://gist.github.com/username/gist_id
    // - https://gist.github.com/gist_id
    // - gist.github.com/username/gist_id

    let url = url.trim_end_matches('/');
    let parts: Vec<&str> = url.split('/').collect();

    if parts.len() < 2 {
        anyhow::bail!("Invalid gist URL format");
    }

    // Get the last part which should be the gist ID
    let gist_id = parts
        .last()
        .ok_or_else(|| anyhow::anyhow!("Could not extract gist ID from URL"))?;

    // Validate it looks like a gist ID (hexadecimal string)
    if gist_id.len() < 5 || !gist_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        anyhow::bail!("Invalid gist ID format");
    }

    Ok(gist_id.to_string())
}

/// Create a new gist with the given content
pub fn create_gist(content: &str, filename: &str, private: bool) -> Result<(String, String)> {
    let mut args = vec!["gist", "create", "-f", filename, "-"];
    if !private {
        args.push("--public");
    }
    // Note: gists are secret by default, so we only add --public flag

    let mut child = Command::new("gh")
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Write content to stdin
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create gist: {}", stderr);
    }

    let gist_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let gist_id = extract_gist_id(&gist_url)?;

    Ok((gist_id, gist_url))
}

/// Update an existing gist
pub fn update_gist(gist_id: &str, content: &str, filename: &str) -> Result<()> {
    let mut child = Command::new("gh")
        .args(&["gist", "edit", gist_id, "-f", filename, "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Write content to stdin
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to update gist: {}", stderr);
    }

    Ok(())
}

/// Download gist content
pub fn download_gist(url: &str) -> Result<String> {
    let output = Command::new("gh")
        .args(&["gist", "view", url, "--raw"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") {
            anyhow::bail!("Gist not found: {}", url);
        } else if stderr.contains("authentication") {
            anyhow::bail!(
                "Cannot access gist (may be private). Ensure you're authenticated: gh auth login"
            );
        } else {
            anyhow::bail!("Failed to download gist: {}", stderr.trim());
        }
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
