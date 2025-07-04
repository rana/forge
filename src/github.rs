use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug)]
struct ScoredAsset {
    asset: Asset,
    score: i32,
}

#[derive(Debug, Clone)]
struct ExecutableInfo {
    name: String,
    path: String,
}

pub struct DiscoveryResult {
    pub download_url: String,
    pub version: String,
    pub asset_name: String,
}

pub struct InstallResult {
    pub version: String,
    pub executables: Vec<String>,
}

pub fn discover_asset(repo: &str, os: &str, arch: &str) -> Result<DiscoveryResult> {
    println!("🔍 Discovering assets for {} ({}-{})", repo, os, arch);

    // Get latest release from GitHub API
    let output = Command::new("gh")
        .args(&["api", &format!("repos/{}/releases/latest", repo)])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to fetch release info for {}", repo);
    }

    let release: Release = serde_json::from_slice(&output.stdout)?;

    if release.assets.is_empty() {
        anyhow::bail!("No assets found in latest release for {}", repo);
    }

    // Score each asset
    let mut scored_assets: Vec<ScoredAsset> = release
        .assets
        .into_iter()
        .filter_map(|asset| score_asset(&asset, os, arch).map(|score| ScoredAsset { asset, score }))
        .collect();

    // Sort by score (highest first)
    scored_assets.sort_by(|a, b| b.score.cmp(&a.score));

    if let Some(best) = scored_assets.first() {
        if best.score > 0 {
            println!("  Found: {} (score: {})", best.asset.name, best.score);
            return Ok(DiscoveryResult {
                download_url: best.asset.browser_download_url.clone(),
                version: release.tag_name.trim_start_matches('v').to_string(),
                asset_name: best.asset.name.clone(),
            });
        }
    }

    // No good match found - provide helpful error
    let asset_names: Vec<String> = scored_assets
        .iter()
        .take(10) // Show only top 10
        .map(|sa| format!("  - {} (score: {})", sa.asset.name, sa.score))
        .collect();

    anyhow::bail!(
        "Could not auto-detect download for {}-{}.\n\
        Found these assets:\n{}\n\n\
        Add an explicit pattern to your tool configuration:\n\
        pattern = \"PATTERN_HERE\"",
        os,
        arch,
        asset_names.join("\n")
    )
}

fn score_asset(asset: &Asset, os: &str, arch: &str) -> Option<i32> {
    let name = asset.name.to_lowercase();
    let mut score = 0;

    // Skip non-downloadable files
    if name.ends_with(".sig")
        || name.ends_with(".asc")
        || name.ends_with(".sha256")
        || name.ends_with(".sha512")
        || name.ends_with(".md5")
        || name.contains(".sha256sum")
    {
        return None;
    }

    // Skip package formats (should use native installers)
    if name.ends_with(".deb")
        || name.ends_with(".rpm")
        || name.ends_with(".dmg")
        || name.ends_with(".msi")
    {
        return None;
    }

    // Skip source archives
    if name.contains("source") || name.contains("src") {
        return None;
    }

    // OS matching
    let os_patterns = match os {
        "linux" => vec!["linux", "Linux"],
        "macos" => vec!["darwin", "Darwin", "macos", "macOS", "osx"],
        "windows" => vec!["windows", "Windows", "win"],
        _ => vec![os],
    };

    let has_os_match = os_patterns
        .iter()
        .any(|pattern| name.contains(&pattern.to_lowercase()));
    if !has_os_match {
        // Check if it's a universal binary (no OS in name might mean universal)
        if !name.contains("linux")
            && !name.contains("darwin")
            && !name.contains("windows")
            && !name.contains("macos")
            && !name.contains("win")
        {
            score += 1; // Low score for potential universal binary
        } else {
            return None; // Wrong OS
        }
    } else {
        score += 10;
    }

    // Architecture matching
    let arch_patterns = match arch {
        "x86_64" => vec!["x86_64", "x64", "amd64", "x86-64"],
        "aarch64" => vec!["aarch64", "arm64"],
        _ => vec![arch],
    };

    let has_arch_match = arch_patterns.iter().any(|pattern| name.contains(pattern));
    if has_arch_match {
        score += 10;
    } else if name.contains("universal") || name.contains("all") {
        score += 5; // Universal binary
    }
    // Note: No architecture in name is often OK

    // Prefer archives over raw binaries (but both are fine)
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        score += 5;
    } else if name.ends_with(".zip") {
        score += 4;
    } else if name.ends_with(".tar.xz") {
        score += 3;
    } else if name.ends_with(".tar.bz2") {
        score += 2;
    }
    // Raw binary gets no bonus but is still valid

    // Prefer release builds over debug
    if name.contains("debug") {
        score -= 10;
    }

    // Shorter names are usually better
    score -= (name.len() / 20) as i32;

    Some(score)
}

pub fn download_and_install(
    url: &str,
    asset_name: &str,
    tool_name: &str,
    provides_hint: &[String],
) -> Result<InstallResult> {
    // Ensure ~/.local/bin exists
    std::fs::create_dir_all(
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("No home directory"))?
            .join(".local/bin"),
    )?;

    let install_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("No home directory"))?
        .join(".local/bin");

    // Determine if it's an archive or raw binary
    let is_archive = asset_name.ends_with(".tar.gz")
        || asset_name.ends_with(".tgz")
        || asset_name.ends_with(".zip")
        || asset_name.ends_with(".tar.xz")
        || asset_name.ends_with(".tar.bz2");

    if is_archive {
        // Download to temp file
        let temp_path = format!("/tmp/{}", asset_name);
        println!("  Downloading archive to {}", temp_path);

        let status = Command::new("curl")
            .args(&["-L", "-o", &temp_path, url])
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to download {}", url);
        }

        // Extract and get list of installed executables
        let executables = extract_and_install(
            &temp_path,
            &asset_name,
            tool_name,
            &install_dir,
            provides_hint,
        )?;

        // Clean up
        std::fs::remove_file(&temp_path).ok();

        Ok(InstallResult {
            version: String::new(), // Will be filled by caller
            executables,
        })
    } else {
        // Raw binary - download directly to install location
        let install_path = install_dir.join(tool_name);
        println!("  Downloading binary to {}", install_path.display());

        let status = Command::new("curl")
            .args(&["-L", "-o", install_path.to_str().unwrap(), url])
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to download {}", url);
        }

        // Make executable
        Command::new("chmod")
            .args(&["+x", install_path.to_str().unwrap()])
            .status()?;

        Ok(InstallResult {
            version: String::new(),
            executables: vec![tool_name.to_string()],
        })
    }
}

fn extract_and_install(
    archive_path: &str,
    archive_name: &str,
    tool_name: &str,
    install_dir: &Path,
    provides_hint: &[String],
) -> Result<Vec<String>> {
    println!("  Extracting archive...");

    if archive_name.ends_with(".tar.gz") || archive_name.ends_with(".tgz") {
        extract_tar(archive_path, tool_name, install_dir, "z", provides_hint)
    } else if archive_name.ends_with(".tar.xz") {
        extract_tar(archive_path, tool_name, install_dir, "J", provides_hint)
    } else if archive_name.ends_with(".tar.bz2") {
        extract_tar(archive_path, tool_name, install_dir, "j", provides_hint)
    } else if archive_name.ends_with(".zip") {
        extract_zip(archive_path, tool_name, install_dir, provides_hint)
    } else {
        anyhow::bail!("Unsupported archive format: {}", archive_name)
    }
}

fn extract_tar(
    archive_path: &str,
    tool_name: &str,
    install_dir: &Path,
    compression_flag: &str,
    provides_hint: &[String],
) -> Result<Vec<String>> {
    // List contents
    let output = Command::new("tar")
        .args(&[&format!("-t{}f", compression_flag), archive_path])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to list tar contents");
    }

    let contents = String::from_utf8_lossy(&output.stdout);

    // Find all executables, using hints if available
    let executables = find_all_executables(&contents, tool_name, provides_hint)?;

    if executables.is_empty() {
        anyhow::bail!("No executables found in archive");
    }

    println!(
        "  Found executables: {}",
        executables
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    );

    // Extract to temp dir
    let temp_dir = format!("/tmp/forge-extract-{}", std::process::id());
    std::fs::create_dir_all(&temp_dir)?;

    Command::new("tar")
        .args(&[
            &format!("-x{}f", compression_flag),
            archive_path,
            "-C",
            &temp_dir,
        ])
        .status()?;

    // Install each executable
    let mut installed = Vec::new();
    for exe in executables {
        let source = Path::new(&temp_dir).join(&exe.path);
        let dest = install_dir.join(&exe.name);

        std::fs::copy(&source, &dest)?;

        // Make executable
        Command::new("chmod")
            .args(&["+x", dest.to_str().unwrap()])
            .status()?;

        installed.push(exe.name);
    }

    // Clean up
    std::fs::remove_dir_all(&temp_dir).ok();

    Ok(installed)
}

fn extract_zip(
    archive_path: &str,
    tool_name: &str,
    install_dir: &Path,
    provides_hint: &[String],
) -> Result<Vec<String>> {
    // List contents
    let output = Command::new("unzip").args(&["-l", archive_path]).output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to list zip contents");
    }

    let contents = String::from_utf8_lossy(&output.stdout);
    let executables = find_all_executables(&contents, tool_name, provides_hint)?;

    if executables.is_empty() {
        anyhow::bail!("No executables found in archive");
    }

    println!(
        "  Found executables: {}",
        executables
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    );

    // Extract to temp dir
    let temp_dir = format!("/tmp/forge-extract-{}", std::process::id());
    std::fs::create_dir_all(&temp_dir)?;

    Command::new("unzip")
        .args(&["-q", archive_path, "-d", &temp_dir])
        .status()?;

    // Install each executable
    let mut installed = Vec::new();
    for exe in executables {
        let source = Path::new(&temp_dir).join(&exe.path);
        let dest = install_dir.join(&exe.name);

        std::fs::copy(&source, &dest)?;

        // Make executable
        Command::new("chmod")
            .args(&["+x", dest.to_str().unwrap()])
            .status()?;

        installed.push(exe.name);
    }

    // Clean up
    std::fs::remove_dir_all(&temp_dir).ok();

    Ok(installed)
}

fn find_all_executables(
    contents: &str,
    tool_name: &str,
    provides_hint: &[String],
) -> Result<Vec<ExecutableInfo>> {
    let mut candidates = Vec::new();

    // First pass: collect all potential executables
    for line in contents.lines() {
        if line.trim().is_empty() || line.ends_with('/') {
            continue;
        }

        let path = line.trim();
        let file_path = Path::new(path);

        if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
            // Skip non-executables
            if name.starts_with('.')
                || name.ends_with(".md")
                || name.ends_with(".txt")
                || name.ends_with(".1")
                || name.ends_with(".fish")
                || name.ends_with(".bash")
                || name.ends_with(".zsh")
                || name.ends_with(".ps1")
                || path.contains("/doc/")
                || path.contains("/docs/")
                || path.contains("/complete/")
                || path.contains("/completion")
                || name.to_lowercase() == "license"
                || name.to_lowercase() == "copying"
                || name.to_lowercase() == "unlicense"
                || name.to_lowercase() == "readme"
                || name.to_lowercase().starts_with("license")
                || name.to_lowercase().starts_with("changelog")
                || name.to_lowercase().starts_with("authors")
            {
                continue;
            }

            // Look for executable patterns
            if !name.contains('.') || name.ends_with(".exe") {
                // Check depth and location
                let depth = file_path.components().count();
                if depth <= 3 && !path.contains("/test") {
                    candidates.push(ExecutableInfo {
                        name: name.to_string(),
                        path: path.to_string(),
                    });
                }
            }
        }
    }

    // Second pass: prioritize based on hints and heuristics
    let mut selected = Vec::new();

    // If we have hints, try to find those first
    if !provides_hint.is_empty() {
        for hint in provides_hint {
            if let Some(exe) = candidates.iter().find(|e| &e.name == hint) {
                selected.push(exe.clone());
            }
        }
    }

    // If no hints or hints didn't match, use heuristics
    if selected.is_empty() {
        // Sort by priority:
        // 1. Exact tool name match
        // 2. Shortest name (often the main executable)
        // 3. In root or bin directory
        candidates.sort_by(|a, b| {
            // Exact match gets priority
            if a.name == tool_name && b.name != tool_name {
                return std::cmp::Ordering::Less;
            }
            if b.name == tool_name && a.name != tool_name {
                return std::cmp::Ordering::Greater;
            }

            // Then prefer shorter names
            a.name.len().cmp(&b.name.len())
        });

        // Take the best candidate
        if let Some(best) = candidates.first() {
            selected.push(best.clone());
        }
    }

    if selected.is_empty() {
        // Show what we found for debugging
        let all_files: Vec<String> = candidates
            .iter()
            .map(|c| format!("  {} ({})", c.name, c.path))
            .collect();

        anyhow::bail!(
            "Could not determine which executable to install.\nCandidates:\n{}",
            all_files.join("\n")
        )
    }

    Ok(selected)
}
