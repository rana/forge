use crate::{
    backend::execute_install,
    color::{ACTION, Colors, INFO, SEARCH, SUCCESS, WARNING},
    facts::{Facts, ToolFact},
    knowledge::{Knowledge, Tool},
    platform::Platform,
    version::check_latest_version,
};
use anyhow::Result;
use chrono::Utc;
use std::process::Command;

pub struct Forge {
    knowledge: Knowledge,
    platform: Platform,
}

impl Forge {
    pub async fn new() -> Result<Self> {
        let knowledge = Knowledge::load().await?;
        let platform = Platform::detect()?;
        Ok(Self {
            knowledge,
            platform,
        })
    }

    pub async fn install(&self, tool_name: &str, installer_name: Option<&str>) -> Result<()> {
        println!("{} Installing {}...", INFO, Colors::info(tool_name));

        // Load facts
        let mut facts = Facts::load().await?;

        // Check if already installed
        if let Some(fact) = facts.tools.get(tool_name) {
            // Check if we're trying to use a different installer
            if let Some(requested_installer) = installer_name {
                if requested_installer != fact.installer {
                    // User explicitly wants a different installer
                    println!(
                        "{} {} is already installed via {} (v{})",
                        WARNING,
                        tool_name,
                        Colors::warning(&fact.installer),
                        Colors::muted(fact.version.as_deref().unwrap_or("unknown"))
                    );
                    println!(
                        "{} Switching to {} installer...",
                        ACTION,
                        Colors::action(requested_installer)
                    );

                    // Uninstall the old version first
                    println!(
                        "{} Uninstalling {} ({})...",
                        ACTION,
                        Colors::warning(tool_name),
                        fact.installer
                    );

                    // Perform uninstall (it handles facts removal)
                    self.uninstall(tool_name).await?;

                    // Restore the fact if uninstall fails
                    // (uninstall removes it from facts, but we already removed it)

                    println!("{} Uninstalled {}", SUCCESS, Colors::success(tool_name));
                    // Continue with installation below
                } else {
                    // Same installer requested - skip
                    println!(
                        "{} {} is already installed via {} (v{})",
                        SUCCESS,
                        tool_name,
                        Colors::info(&fact.installer),
                        Colors::muted(fact.version.as_deref().unwrap_or("unknown"))
                    );
                    return Ok(());
                }
            } else {
                // No specific installer requested - keep existing
                println!(
                    "{} {} is already installed (v{})",
                    SUCCESS,
                    tool_name,
                    Colors::muted(fact.version.as_deref().unwrap_or("unknown"))
                );
                return Ok(());
            }
        }

        // Find tool
        let tool = self
            .knowledge
            .tools
            .get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;

        // Find installer - with platform awareness
        let (installer_key, tool_installer) = if let Some(name) = installer_name {
            tool.installers
                .get(name)
                .map(|ti| (name.to_string(), ti))
                .ok_or_else(|| {
                    anyhow::anyhow!("{} doesn't support installer: {}", tool_name, name)
                })?
        } else {
            // Use platform precedence
            self.find_best_installer(tool_name, tool)?
        };

        let installer = self
            .knowledge
            .installers
            .get(&installer_key)
            .ok_or_else(|| anyhow::anyhow!("Unknown installer: {}", installer_key))?;

        println!(
            "{} Using {} installer",
            ACTION,
            Colors::action(&installer_key)
        );

        // Check if installer is available (skip for script installers)
        if installer.installer_type != "script" {
            if let Some(check) = &installer.check {
                let result = Command::new(&check[0]).args(&check[1..]).output();

                if result.is_err() || !result.unwrap().status.success() {
                    // Look for a tool that provides this installer
                    if let Some(provider) = self.find_tool_that_provides(&installer_key) {
                        println!(
                            "\n{} {} installer not available",
                            crate::color::ERROR,
                            installer_key
                        );
                        println!(
                            "\n{} {} is provided by: {}",
                            crate::color::TIP,
                            installer_key,
                            Colors::info(&provider.0)
                        );
                        println!("   {}", Colors::muted(&provider.1.description));
                        println!("\nInstall it with:");
                        println!(
                            "   {}",
                            Colors::action(&format!("forge install {}", provider.0))
                        );

                        anyhow::bail!("Missing installer");
                    } else {
                        anyhow::bail!(
                            "{} installer not available. Please install it first.",
                            installer_key
                        );
                    }
                }
            }
        }

        // Execute installation and capture version
        let result = if installer.installer_type == "script" {
            // For script installers, get the platform-specific script
            let platform_scripts = match self.platform.os.as_str() {
                "linux" => &tool_installer.linux,
                "macos" => &tool_installer.macos,
                "windows" => &tool_installer.windows,
                _ => {
                    anyhow::bail!("Platform {} not supported", self.platform.os);
                }
            };

            let scripts = platform_scripts.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No script for {} on {}", tool_name, self.platform.os)
            })?;

            crate::backend::execute_script_install(&scripts.install, tool_name, &self.platform)?
        } else if installer_key == "github" {
            // Use smart GitHub installer
            crate::backend::execute_github_install(tool_name, tool_installer, tool, &self.platform)?
        } else {
            execute_install(installer, tool_name, tool_installer, None, &self.platform)?
        };

        // Record in facts
        facts.tools.insert(
            tool_name.to_string(),
            ToolFact {
                installed_at: Utc::now(),
                installer: installer_key.clone(),
                version: if installer.installer_type == "script" {
                    None // Don't record version for script installers
                } else {
                    Some(result.version.clone())
                },
                executables: result.executables.clone(),
            },
        );
        facts.save().await?;

        // Success message
        if installer.installer_type == "script" {
            println!(
                "{} {} installed successfully!",
                SUCCESS,
                Colors::success(tool_name)
            );

            // Show what commands are now available
            if let Some(tool) = self.knowledge.tools.get(tool_name) {
                if !tool.provides.is_empty() {
                    println!("\nNow available:");
                    for cmd in &tool.provides {
                        // Check if command exists and show version if possible
                        if let Ok(output) = Command::new(cmd).arg("--version").output() {
                            if output.status.success() {
                                let version_output = String::from_utf8_lossy(&output.stdout);
                                let first_line = version_output.lines().next().unwrap_or("");
                                println!("  • {}", Colors::muted(first_line));
                            }
                        }
                    }
                }
            }
        } else {
            println!(
                "{} {} v{} installed successfully!",
                SUCCESS,
                Colors::success(tool_name),
                Colors::warning(&result.version)
            );
        }

        Ok(())
    }

    pub async fn update(&self, tool_name: Option<&str>, tools_only: bool) -> Result<()> {
        let facts = Facts::load().await?;

        if facts.tools.is_empty() {
            println!("{}", Colors::muted("No tools installed yet."));
            return Ok(());
        }

        let tools_to_check: Vec<(String, ToolFact)> = if let Some(name) = tool_name {
            if let Some(fact) = facts.tools.get(name) {
                vec![(name.to_string(), fact.clone())]
            } else {
                anyhow::bail!("{} is not installed", name);
            }
        } else {
            facts
                .tools
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };

        println!("{} Checking for updates...", SEARCH);

        let mut updates = Vec::new();

        for (name, fact) in &tools_to_check {
            if let Some(tool) = self.knowledge.tools.get(name) {
                let current = fact.version.as_deref().unwrap_or("unknown");

                // Check latest version
                let installer = self.knowledge.installers.get(&fact.installer);
                let tool_installer = tool.installers.get(&fact.installer);
                let package = tool_installer
                    .and_then(|ti| ti.package.as_ref())
                    .unwrap_or(name);

                let latest = if let Some(inst) = installer {
                    check_latest_version(&fact.installer, package, inst.version_check.as_ref())
                        .await?
                } else {
                    None
                };

                let has_update = match (&fact.version, &latest) {
                    (Some(c), Some(l)) => c != l,
                    _ => false,
                };

                if has_update {
                    println!(
                        "  {} {} → {}",
                        Colors::info(name),
                        Colors::muted(current),
                        Colors::success(latest.as_deref().unwrap_or("unknown"))
                    );
                    updates.push((name.clone(), fact.installer.clone(), latest));
                } else {
                    println!(
                        "  {} {} {}",
                        Colors::info(name),
                        Colors::muted(current),
                        Colors::muted("(up to date)")
                    );
                }
            }
        }

        if updates.is_empty() {
            println!("\n{} All tools are up to date!", SUCCESS);
            return Ok(());
        }

        // Show summary of updates
        println!(
            "\n{} {} {} available",
            INFO,
            updates.len(),
            if updates.len() == 1 {
                "update"
            } else {
                "updates"
            }
        );

        // Update package managers first (unless --tools-only)
        if !tools_only {
            println!("\n{} Updating package managers...", ACTION);

            // Find unique installers used by installed tools
            let mut installers_to_update = std::collections::HashSet::new();
            for (_, fact) in &facts.tools {
                installers_to_update.insert(&fact.installer);
            }

            // Update each installer's provider
            for installer_name in installers_to_update {
                if let Some(provider_tool) = self.find_tool_that_provides(installer_name) {
                    if let Some(installer) = self.knowledge.installers.get(installer_name) {
                        if installer.update.is_some() {
                            println!(
                                "  {} Updating {} (provides {})",
                                ACTION,
                                Colors::info(&provider_tool.0),
                                installer_name
                            );

                            // Execute the update
                            if let Err(e) = self
                                .execute_installer_update(&provider_tool.0, installer_name)
                                .await
                            {
                                println!(
                                    "  {} Failed to update {}: {}",
                                    WARNING, provider_tool.0, e
                                );
                            }
                        }
                    }
                }
            }
        }

        // Perform updates
        for (tool_name, installer_name, _version) in updates {
            println!("\n{} Updating {}...", ACTION, Colors::info(&tool_name));

            // Uninstall old version first if uninstall command exists
            if let Some(installer) = self.knowledge.installers.get(&installer_name) {
                if installer.uninstall.is_some() {
                    self.uninstall(&tool_name).await?;
                }
            }

            // Install new version
            self.install(&tool_name, Some(&installer_name)).await?;
        }

        println!("\n{} Updates complete!", SUCCESS);
        Ok(())
    }

    pub async fn uninstall(&self, tool_name: &str) -> Result<()> {
        println!(
            "{} Preparing to uninstall {}...",
            ACTION,
            Colors::info(tool_name)
        );

        let mut facts = Facts::load().await?;

        if let Some(fact) = facts.tools.get(tool_name) {
            let tool = self.knowledge.tools.get(tool_name);
            let provides: &[_] = tool.as_ref().map_or(&[], |t| &t.provides);

            // Check if this tool provides any installers
            if !provides.is_empty() {
                // Find all tools installed by the installers this tool provides
                let dependent_tools: Vec<String> = facts
                    .tools
                    .iter()
                    .filter(|(name, f)| *name != tool_name && provides.contains(&f.installer))
                    .map(|(name, _)| name.clone())
                    .collect();

                if !dependent_tools.is_empty() {
                    println!(
                        "\n{} {} provides the {} installer",
                        WARNING,
                        tool_name,
                        provides.join(", ")
                    );
                    println!("The following tools were installed using it:");
                    for dep in &dependent_tools {
                        println!("  • {}", Colors::info(dep));
                    }
                    println!("\nThese tools will be removed from Forge's records.");
                    println!("(The actual binaries may also be removed by the uninstaller)");
                }
            }

            // No confirmation needed - trust the user
            println!(
                "\n{} Uninstalling {}...",
                ACTION,
                Colors::warning(tool_name)
            );

            // Remove the actual executables first
            if let Some(executables) = &fact.executables {
                for exe in executables {
                    let exe_path = dirs::home_dir()
                        .ok_or_else(|| anyhow::anyhow!("No home directory"))?
                        .join(".local/bin")
                        .join(exe);

                    if exe_path.exists() {
                        println!("  {} Removing executable: {}", ACTION, exe);
                        std::fs::remove_file(&exe_path)?;
                    }
                }
            }

            // Try to use uninstall command if available
            if let Some(installer) = self.knowledge.installers.get(&fact.installer) {
                if let Some(uninstall_cmd) = &installer.uninstall {
                    let default = Default::default();
                    let tool_config = self
                        .knowledge
                        .tools
                        .get(tool_name)
                        .and_then(|t| t.installers.get(&fact.installer))
                        .unwrap_or(&default);

                    let mut command = uninstall_cmd.clone();
                    for part in &mut command {
                        *part = crate::backend::expand_template(
                            part,
                            tool_name,
                            tool_config,
                            None,
                            &self.platform,
                        );
                    }

                    println!("{} Running: {}", ACTION, Colors::muted(&command.join(" ")));
                    let output = Command::new(&command[0]).args(&command[1..]).output()?;

                    if !output.status.success() {
                        println!("{} Uninstall command failed", WARNING);
                    }
                } else if installer.installer_type == "script" {
                    // Use platform-specific uninstall script if available
                    if let Some(tool_def) = self.knowledge.tools.get(tool_name) {
                        if let Some(tool_installer) = tool_def.installers.get(&fact.installer) {
                            let platform_scripts = match self.platform.os.as_str() {
                                "linux" => &tool_installer.linux,
                                "macos" => &tool_installer.macos,
                                "windows" => &tool_installer.windows,
                                _ => &None,
                            };

                            if let Some(scripts) = platform_scripts {
                                if let Some(uninstall_script) = &scripts.uninstall {
                                    println!("{} Running uninstall script...", ACTION);
                                    let output = Command::new("sh")
                                        .arg("-c")
                                        .arg(uninstall_script)
                                        .output()?;

                                    if !output.status.success() {
                                        println!("{} Uninstall script failed", WARNING);
                                    }
                                } else {
                                    println!("{} No uninstaller available for {}", INFO, tool_name);
                                }
                            }
                        }
                    }
                }
            }

            // Remove from facts
            facts.tools.remove(tool_name);

            // Also remove tools that were installed by this tool's installers
            if !provides.is_empty() {
                let tools_to_remove: Vec<String> = facts
                    .tools
                    .iter()
                    .filter(|(_, f)| provides.contains(&f.installer))
                    .map(|(name, _)| name.clone())
                    .collect();

                for tool in tools_to_remove {
                    println!("{} Removing {} from records", ACTION, Colors::muted(&tool));
                    facts.tools.remove(&tool);
                }
            }

            facts.save().await?;

            println!("{} {} uninstalled", SUCCESS, Colors::success(tool_name));
        } else {
            println!("{} {} is not installed", INFO, tool_name);
        }

        Ok(())
    }

    pub fn why(&self, tool_name: &str) -> Result<()> {
        let tool = self
            .knowledge
            .tools
            .get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;

        println!("{}", Colors::info(tool_name));
        println!("{}", Colors::muted(&tool.description));

        Ok(())
    }

    pub async fn list(&self) -> Result<()> {
        let facts = Facts::load().await?;

        if facts.tools.is_empty() {
            println!("{}", Colors::muted("No tools installed yet."));
            println!("\nGet started with:");
            println!("  {}", Colors::action("forge install ripgrep"));
            return Ok(());
        }

        // Calculate column widths
        let mut max_name_len = 0;
        let mut max_version_len = 0;
        let mut max_installer_len = 0;

        for (name, fact) in &facts.tools {
            max_name_len = max_name_len.max(name.len());
            let version_len = fact.version.as_deref().unwrap_or("unknown").len() + 1; // +1 for 'v' prefix
            max_version_len = max_version_len.max(version_len);
            max_installer_len = max_installer_len.max(fact.installer.len());
        }

        println!("Installed tools:");
        for (name, fact) in &facts.tools {
            let tool = self.knowledge.tools.get(name);
            let description = tool
                .map(|t| t.description.as_str())
                .unwrap_or("Unknown tool");
            let version = fact.version.as_deref().unwrap_or("unknown");

            // Add (local) marker if from local overlay
            let local_marker = if self.knowledge.local_tools.contains(name) {
                " (local)"
            } else {
                ""
            };

            println!(
                "  • {}{} {} - {}",
                Colors::info(&name),
                Colors::muted(local_marker),
                Colors::muted(&format!("v{}", version)),
                Colors::muted(description)
            );
        }

        Ok(())
    }

    pub async fn fmt(&self, file: Option<&str>, check: bool) -> Result<()> {
        use crate::format::{find_knowledge_files, format_toml};

        println!("{} Formatting TOML files...", INFO);

        let files = find_knowledge_files(file).await?;
        let mut all_formatted = true;

        for file in files {
            let formatted = format_toml(&file, check).await?;
            if !formatted {
                all_formatted = false;
            }
        }

        if check && !all_formatted {
            anyhow::bail!("Some files need formatting");
        }

        Ok(())
    }

    async fn execute_installer_update(&self, tool_name: &str, installer_name: &str) -> Result<()> {
        // For script installers, use platform-specific update script
        if installer_name == "script" {
            if let Some(tool) = self.knowledge.tools.get(tool_name) {
                if let Some(tool_installer) = tool.installers.get("script") {
                    let platform_scripts = match self.platform.os.as_str() {
                        "linux" => &tool_installer.linux,
                        "macos" => &tool_installer.macos,
                        "windows" => &tool_installer.windows,
                        _ => return Ok(()),
                    };

                    if let Some(scripts) = platform_scripts {
                        if let Some(update_script) = &scripts.update {
                            let output =
                                Command::new("sh").arg("-c").arg(update_script).output()?;

                            if !output.status.success() {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                anyhow::bail!("Update script failed: {}", stderr);
                            }
                        }
                    }
                }
            }
        } else {
            // For command installers, use the update command if available
            if let Some(installer) = self.knowledge.installers.get(installer_name) {
                if let Some(update_cmd) = &installer.update {
                    if let Some(tool) = self.knowledge.tools.get(tool_name) {
                        if let Some(tool_installer) = tool.installers.get(installer_name) {
                            let mut command = update_cmd.clone();
                            for part in &mut command {
                                *part = crate::backend::expand_template(
                                    part,
                                    tool_name,
                                    tool_installer,
                                    None,
                                    &self.platform,
                                );
                            }

                            let output = Command::new(&command[0]).args(&command[1..]).output()?;

                            if !output.status.success() {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                anyhow::bail!("Update command failed: {}", stderr);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn find_best_installer<'a>(
        &self,
        tool_name: &str,
        tool: &'a crate::knowledge::Tool,
    ) -> Result<(String, &'a crate::knowledge::ToolInstaller)> {
        // Get platform precedence
        let platform_name = &self.platform.os;
        let precedence = self
            .knowledge
            .platforms
            .get(platform_name)
            .map(|p| &p.precedence)
            .ok_or_else(|| anyhow::anyhow!("No platform config for {}", platform_name))?;

        // Find first available installer in precedence order
        for installer_name in precedence {
            if let Some(tool_installer) = tool.installers.get(installer_name) {
                // Also verify the installer itself exists in knowledge
                if self.knowledge.installers.contains_key(installer_name) {
                    return Ok((installer_name.clone(), tool_installer));
                }
            }
        }

        // If no installer found in precedence, list what's available
        let available: Vec<&str> = tool.installers.keys().map(|s| s.as_str()).collect();
        anyhow::bail!(
            "No installer available for {} on {}. Tool supports: {:?}",
            tool_name,
            platform_name,
            available
        )
    }

    fn find_tool_that_provides(&self, command: &str) -> Option<(String, &Tool)> {
        self.knowledge
            .tools
            .iter()
            .find(|(_, tool)| tool.provides.contains(&command.to_string()))
            .map(|(name, tool)| (name.clone(), tool))
    }
}
