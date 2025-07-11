use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Knowledge {
    pub version: u32,
    pub installers: HashMap<String, Installer>,
    pub tools: HashMap<String, Tool>,
    pub platforms: HashMap<String, PlatformConfig>,
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
        let bundled = include_str!("../data/knowledge.toml");
        let knowledge: Knowledge = toml::from_str(bundled)?;
        Ok(knowledge)
    }
}
