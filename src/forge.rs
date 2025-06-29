use crate::{
    backend::execute_install,
    color::{ACTION, Colors, INFO, SEARCH, SUCCESS, TIP, WARNING},
    facts::{Facts, ToolFact},
    knowledge::Knowledge,
    platform::Platform,
    version::{check_latest_version, detect_installed_version},
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
            println!(
                "{} {} is already installed (v{})",
                SUCCESS,
                tool_name,
                Colors::muted(fact.version.as_deref().unwrap_or("unknown"))
            );
            return Ok(());
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
            // Find first available installer for this platform
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

        // Check if installer is available
        if let Some(check) = &installer.check {
            let result = Command::new(&check[0]).args(&check[1..]).output();

            if result.is_err() || !result.unwrap().status.success() {
                anyhow::bail!(
                    "{} installer not available. Please install it first.",
                    installer_key
                );
            }
        }

        // Execute installation
        let result = execute_install(installer, tool_name, tool_installer, None, &self.platform)?;

        // Record in facts
        facts.tools.insert(
            tool_name.to_string(),
            ToolFact {
                installed_at: Utc::now(),
                installer: installer_key,
                version: Some(result.version.clone()),
            },
        );
        facts.save().await?;

        println!(
            "{} {} v{} installed successfully!",
            SUCCESS,
            Colors::success(tool_name),
            Colors::warning(&result.version)
        );
        Ok(())
    }

    pub async fn update(&self, tool_name: Option<&str>) -> Result<()> {
        let facts = Facts::load().await?;

        if facts.tools.is_empty() {
            println!("{}", Colors::muted("No tools installed yet."));
            return Ok(());
        }

        let tools_to_check: Vec<(String, ToolFact)> = if let Some(name) = tool_name {
            // Update specific tool
            if let Some(fact) = facts.tools.get(name) {
                vec![(name.to_string(), fact.clone())]
            } else {
                anyhow::bail!("{} is not installed", name);
            }
        } else {
            // Update all tools
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
                // Get version patterns
                let patterns = if let Some(tool_patterns) = &tool.version_detection {
                    vec![tool_patterns.clone()]
                } else {
                    self.knowledge.version_detection.default.clone()
                };

                // Check current version
                let current = detect_installed_version(name, &patterns)?;

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

                let has_update = match (&current, &latest) {
                    (Some(c), Some(l)) => c != l,
                    _ => false,
                };

                if has_update {
                    println!(
                        "  {} {} → {}",
                        Colors::info(name),
                        Colors::muted(current.as_deref().unwrap_or("unknown")),
                        Colors::success(latest.as_deref().unwrap_or("unknown"))
                    );
                    updates.push((name.clone(), fact.installer.clone(), latest));
                } else {
                    println!(
                        "  {} {} {}",
                        Colors::info(name),
                        Colors::muted(current.as_deref().unwrap_or("unknown")),
                        Colors::muted("(up to date)")
                    );
                }
            }
        }

        if updates.is_empty() {
            println!("\n{} All tools are up to date!", SUCCESS);
            return Ok(());
        }

        // Ask for confirmation
        println!(
            "\n{} {} {} available. Update? [Y/n] ",
            WARNING,
            updates.len(),
            if updates.len() == 1 {
                "update"
            } else {
                "updates"
            }
        );

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Update cancelled.");
            return Ok(());
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
        println!("{} Uninstalling {}...", ACTION, Colors::info(tool_name));

        let mut facts = Facts::load().await?;

        if let Some(fact) = facts.tools.get(tool_name) {
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
                }
            }

            // Remove from facts
            facts.tools.remove(tool_name);
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
            println!("\n{} Try: forge install ripgrep", TIP);
            return Ok(());
        }

        println!("Installed tools:");
        for (name, fact) in &facts.tools {
            let tool = self.knowledge.tools.get(name);
            let desc = tool.as_ref().map_or("", |t| &t.description);

            println!(
                "  • {} {} - {}",
                Colors::info(name),
                Colors::warning(fact.version.as_deref().unwrap_or("")),
                Colors::muted(desc)
            );
        }

        Ok(())
    }

    fn find_best_installer<'a>(
        &self,
        tool_name: &str,
        tool: &'a crate::knowledge::Tool,
    ) -> Result<(String, &'a crate::knowledge::ToolInstaller)> {
        // For now, just return the first one
        // TODO: Add platform precedence from knowledge.toml
        tool.installers
            .iter()
            .next()
            .map(|(k, v)| (k.clone(), v))
            .ok_or_else(|| anyhow::anyhow!("No installers available for {}", tool_name))
    }
}
