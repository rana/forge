use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Knowledge {
    pub version: u32,
    pub installers: HashMap<String, Installer>,
    pub tools: HashMap<String, Tool>,
    pub platforms: HashMap<String, PlatformConfig>,
    #[serde(skip)]
    pub local_tools: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LocalKnowledge {
    #[serde(default)]
    installers: HashMap<String, Installer>,
    #[serde(default)]
    tools: HashMap<String, Tool>,
    #[serde(default)]
    platforms: HashMap<String, PlatformConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformConfig {
    pub precedence: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Installer {
    #[serde(rename = "type")]
    pub installer_type: String,
    pub check: Option<Vec<String>>,
    pub install: Vec<String>,
    pub uninstall: Option<Vec<String>>,
    pub update: Option<Vec<String>>, // NEW
    pub install_output_pattern: Option<String>,
    pub version_check: Option<VersionCheck>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    pub description: String,
    #[serde(default)]
    pub provides: Vec<String>,
    pub installers: HashMap<String, ToolInstaller>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToolInstaller {
    // For command installers
    pub package: Option<String>,
    pub repo: Option<String>,
    pub pattern: Option<String>,
    pub url: Option<String>,

    // For script installers - platform specific
    pub linux: Option<PlatformScripts>,
    pub macos: Option<PlatformScripts>,
    pub windows: Option<PlatformScripts>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformScripts {
    pub install: String,
    pub uninstall: Option<String>,
    pub update: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionCheck {
    pub method: String,
    pub command: Option<Vec<String>>,
    pub url: Option<String>,
    pub path: Option<String>,
}

impl Knowledge {
    pub async fn load() -> Result<Self> {
        // Load embedded knowledge
        let bundled = include_str!("../data/forge.toml");
        let mut knowledge: Knowledge = toml::from_str(bundled)?;
        knowledge.local_tools = HashSet::new(); // Initialize the field

        // Try to load and merge local overlay
        if let Some(local) = Self::load_local().await? {
            knowledge.merge_local(local);
        }

        Ok(knowledge)
    }

    async fn load_local() -> Result<Option<LocalKnowledge>> {
        let path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("No home directory"))?
            .join(".forge")
            .join("forge.toml");

        if !path.exists() {
            return Ok(None);
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match toml::from_str::<LocalKnowledge>(&content) {
                Ok(local) => Ok(Some(local)),
                Err(e) => {
                    eprintln!("⚠️  Warning: Invalid TOML in {}: {}", path.display(), e);
                    eprintln!("   Continuing with embedded knowledge only");
                    Ok(None)
                }
            },
            Err(e) => {
                eprintln!("⚠️  Warning: Could not read {}: {}", path.display(), e);
                eprintln!("   Continuing with embedded knowledge only");
                Ok(None)
            }
        }
    }

        fn merge_local(&mut self, local: LocalKnowledge) {
        // Merge tools - local completely replaces embedded
        for (name, tool) in local.tools {
            self.local_tools.insert(name.clone());
            self.tools.insert(name, tool);
        }
        
        // Merge installers - local completely replaces embedded
        for (name, installer) in local.installers {
            self.installers.insert(name, installer);
        }
        
        // Merge platforms - local completely replaces embedded
        for (name, platform) in local.platforms {
            self.platforms.insert(name, platform);
        }

        // Note: version is ignored from local file
    }
}
