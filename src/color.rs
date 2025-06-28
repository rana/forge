use colored::*;

pub struct Colors;

impl Colors {
    pub fn action(s: &str) -> ColoredString {
        s.blue()
    }
    
    pub fn success(s: &str) -> ColoredString {
        s.green()
    }
    
    pub fn warning(s: &str) -> ColoredString {
        s.yellow()
    }
    
    pub fn error(s: &str) -> ColoredString {
        s.red()
    }
    
    pub fn info(s: &str) -> ColoredString {
        s.cyan()
    }
    
    pub fn muted(s: &str) -> ColoredString {
        s.dimmed()
    }
}

// Semantic emoji
pub const ACTION: &str = "⚡";
pub const SUCCESS: &str = "✅";
pub const INFO: &str = "ℹ️";
pub const WARNING: &str = "⚠️";
pub const ERROR: &str = "❌";
pub const TIP: &str = "💡";
pub const SEARCH: &str = "🔍";
