use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Platform {
    pub os: String,
    pub arch: String,
}

impl Platform {
    pub fn detect() -> Result<Self> {
        let os = match env::consts::OS {
            "linux" => "linux",
            "macos" | "darwin" => "macos", 
            "windows" => "windows",
            other => anyhow::bail!("Unsupported OS: {}", other),
        };

        let arch = match env::consts::ARCH {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            other => anyhow::bail!("Unsupported architecture: {}", other),
        };

        Ok(Platform {
            os: os.to_string(),
            arch: arch.to_string(),
        })
    }
    
    pub fn expand_pattern(&self, pattern: &str) -> String {
        pattern
            .replace("{os}", &self.os)
            .replace("{arch}", &self.arch)
            .replace("{target}", &self.target_triple())
    }
    
    fn target_triple(&self) -> String {
        match (self.os.as_str(), self.arch.as_str()) {
            ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
            ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
            ("macos", "x86_64") => "x86_64-apple-darwin",
            ("macos", "aarch64") => "aarch64-apple-darwin",
            _ => "unknown",
        }.to_string()
    }
}
