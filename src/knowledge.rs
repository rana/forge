use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Knowledge {
    pub version: u32,
    pub installers: HashMap<String, Installer>,
    pub tools: HashMap<String, Tool>,
    #[serde(default)]
    pub version_detection: VersionDetection,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Installer {
    #[serde(rename = "type")]
    pub installer_type: String,
    pub check: Option<Vec<String>>,
    pub install: Vec<String>,
    pub uninstall: Option<Vec<String>>,
    pub version_check: Option<VersionCheck>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    pub description: String,
    pub installers: HashMap<String, ToolInstaller>,
    #[serde(default)]
    pub version_detection: Option<VersionPattern>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolInstaller {
    // Only tool-specific overrides
    pub package: Option<String>,
    pub repo: Option<String>,
    pub pattern: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionCheck {
    pub method: String, // "api" or "command"
    pub url: Option<String>,
    pub path: Option<String>, // JSON path for API method
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VersionDetection {
    pub default: Vec<VersionPattern>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionPattern {
    pub command: Vec<String>,
    pub pattern: String,
    pub line: Option<usize>,
}

impl Knowledge {
    pub async fn load() -> Result<Self> {
        let bundled = include_str!("../data/knowledge.toml");
        let knowledge: Knowledge = toml::from_str(bundled)?;
        Ok(knowledge)
    }
}
