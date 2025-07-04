use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Facts {
    #[serde(default)]
    pub tools: HashMap<String, ToolFact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolFact {
    pub installed_at: DateTime<Utc>,
    pub installer: String,
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executables: Option<Vec<String>>,
}


impl Facts {
    pub async fn load() -> Result<Self> {
        let path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("No home directory"))?
            .join(".forge")
            .join("facts.toml");
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(toml::from_str(&content)?)
    }
    
    pub async fn save(&self) -> Result<()> {
        let path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("No home directory"))?
            .join(".forge")
            .join("facts.toml");
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }
}
