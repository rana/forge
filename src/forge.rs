use crate::{
    backend::{execute_install, check_tool_version},
    facts::{Facts, ToolFact},
    knowledge::Knowledge,
};
use anyhow::{Context, Result};
use colored::*;
use chrono::Utc;

pub struct Forge {
    knowledge: Knowledge,
}

impl Forge {
    pub async fn new() -> Result<Self> {
        let knowledge = Knowledge::load().await?;
        Ok(Self { knowledge })
    }
    
    pub async fn install(&self, tool_name: &str, installer_name: Option<&str>) -> Result<()> {
        println!("📦 Installing {}...", tool_name.cyan());
        
        // Load facts
        let mut facts = Facts::load().await?;
        
        // Check if already installed
        if facts.tools.contains_key(tool_name) {
            println!("✓ {} is already installed", tool_name);
            return Ok(());
        }
        
        // Find tool
        let tool = self.knowledge.tools.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;
        
        // Find installer
        let (installer_key, tool_installer) = if let Some(name) = installer_name {
            tool.installers.get(name)
                .map(|ti| (name.to_string(), ti))
                .ok_or_else(|| anyhow::anyhow!("{} doesn't support installer: {}", tool_name, name))?
        } else {
            tool.installers.iter().next()
                .map(|(k, v)| (k.clone(), v))
                .ok_or_else(|| anyhow::anyhow!("No installers available for {}", tool_name))?
        };
        
        let installer = self.knowledge.installers.get(&installer_key)
            .ok_or_else(|| anyhow::anyhow!("Unknown installer: {}", installer_key))?;
        
        // Check if installer is available
        if let Some(check) = &installer.check {
            let result = Command::new(&check[0])
                .args(&check[1..])
                .output();
            
            if result.is_err() || !result.unwrap().status.success() {
                anyhow::bail!("{} installer not available. Please install it first.", installer_key);
            }
        }
        
        // Execute installation
        let result = execute_install(installer, tool_name, tool_installer, None)?;
        
        // Record in facts
        facts.tools.insert(tool_name.to_string(), ToolFact {
            installed_at: Utc::now(),
            installer: installer_key,
            version: Some(result.version.clone()),
        });
        facts.save().await?;
        
        println!("✅ {} v{} installed successfully!", tool_name.cyan(), result.version.yellow());
        Ok(())
    }
    
    pub async fn list(&self) -> Result<()> {
        let facts = Facts::load().await?;
        
        if facts.tools.is_empty() {
            println!("No tools installed yet.");
            return Ok(());
        }
        
        println!("Installed tools:");
        for (name, fact) in &facts.tools {
            let tool = self.knowledge.tools.get(name);
            let desc = tool.map(|t| &t.description).unwrap_or(&String::new());
            
            println!(
                "  • {} {} - {}",
                name.cyan(),
                fact.version.as_deref().unwrap_or("").yellow(),
                desc.dimmed()
            );
        }
        
        Ok(())
    }
}
