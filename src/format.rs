use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use toml::Value;

use crate::color::{Colors, ERROR, SUCCESS};

/// Format a TOML file according to Forge conventions
pub async fn format_toml(path: &Path, check_only: bool) -> Result<bool> {
    // Read the file
    let content = tokio::fs::read_to_string(path).await?;

    // Parse as TOML
    let doc: toml::Value = toml::from_str(&content)?;

    // Format the document
    let formatted = format_document(&doc)?;

    // Check if changes are needed
    if formatted == content {
        if !check_only {
            println!("{} {} is already formatted", SUCCESS, path.display());
        }
        return Ok(true);
    }

    if check_only {
        println!("{} {} needs formatting", ERROR, path.display());
        return Ok(false);
    }

    // Write back
    tokio::fs::write(path, formatted).await?;
    println!(
        "{} Formatted {}",
        SUCCESS,
        Colors::success(&path.display().to_string())
    );

    Ok(true)
}

/// Find knowledge.toml files to format
pub async fn find_knowledge_files(explicit_path: Option<&str>) -> Result<Vec<PathBuf>> {
    if let Some(path) = explicit_path {
        return Ok(vec![PathBuf::from(path)]);
    }

    let mut files = Vec::new();

    // Check current directory
    let local = PathBuf::from("knowledge.toml");
    if local.exists() {
        files.push(local);
    }

    // Check ~/.forge/knowledge.toml
    if let Some(home) = dirs::home_dir() {
        let global = home.join(".forge").join("knowledge.toml");
        if global.exists() {
            files.push(global);
        }
    }

    if files.is_empty() {
        anyhow::bail!(
            "No knowledge.toml found. Specify a file path or run from a directory with knowledge.toml"
        );
    }

    Ok(files)
}

fn format_document(doc: &Value) -> Result<String> {
    let table = doc
        .as_table()
        .ok_or_else(|| anyhow::anyhow!("Invalid TOML: root must be a table"))?;

    let mut output = String::new();

    // 1. Version
    if let Some(version) = table.get("version") {
        output.push_str(&format!("version = {}\n", serialize_value(version)?));
    }
    output.push('\n');

    // 2. Platforms
    if let Some(platforms) = table.get("platforms") {
        output.push_str("# Platforms\n");
        if let Value::Table(platforms_table) = platforms {
            let sorted: BTreeMap<_, _> = platforms_table.iter().collect();
            for (name, config) in sorted {
                output.push_str(&format!("[platforms.{}]\n", name));
                if let Value::Table(config_table) = config {
                    output.push_str(&serialize_table_contents(config_table, &["precedence"])?);
                }
                output.push('\n');
            }
        }
    }

    // 3. Installers
    if let Some(installers) = table.get("installers") {
        output.push_str("# Installers\n");
        if let Value::Table(installers_table) = installers {
            let sorted: BTreeMap<_, _> = installers_table.iter().collect();
            for (name, config) in sorted {
                output.push_str(&format!("[installers.{}]\n", name));
                if let Value::Table(config_table) = config {
                    output.push_str(&serialize_installer_table(config_table)?);
                }
                output.push('\n');
            }
        }
    }

    // 4. Tools
    if let Some(tools) = table.get("tools") {
        output.push_str("# Tools\n");
        if let Value::Table(tools_table) = tools {
            let sorted: BTreeMap<_, _> = tools_table.iter().collect();
            for (name, config) in sorted {
                output.push_str(&serialize_tool(name, config)?);
                output.push('\n');
            }
        }
    }

    // Remove trailing newline
    if output.ends_with("\n\n") {
        output.pop();
    }

    Ok(output)
}

fn serialize_tool(name: &str, value: &Value) -> Result<String> {
    let mut output = format!("[tools.{}]\n", name);

    if let Value::Table(table) = value {
        // First serialize simple properties
        let simple_keys = ["description", "provides"];
        for key in &simple_keys {
            if let Some(val) = table.get(*key) {
                output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
            }
        }

        // Then handle installers
        if let Some(installers) = table.get("installers") {
            if let Value::Table(installers_table) = installers {
                let sorted: BTreeMap<_, _> = installers_table.iter().collect();
                for (installer_name, installer_config) in sorted {
                    output.push('\n');
                    output.push_str(&serialize_tool_installer(
                        name,
                        installer_name,
                        installer_config,
                    )?);
                }
            }
        }
    }

    Ok(output)
}

fn serialize_tool_installer(
    tool_name: &str,
    installer_name: &str,
    config: &Value,
) -> Result<String> {
    let mut output = String::new();

    if let Value::Table(table) = config {
        // For script installers, platform scripts should be at the top level
        let is_script_installer = installer_name == "script";

        if is_script_installer {
            output.push_str(&format!(
                "[tools.{}.installers.{}]\n",
                tool_name, installer_name
            ));

            // Check if we have the old structure with nested scripts
            if let Some(scripts_value) = table.get("scripts") {
                if let Value::Table(scripts_table) = scripts_value {
                    // Flatten the scripts to top level
                    let sorted: BTreeMap<_, _> = scripts_table.iter().collect();
                    for (platform, script) in sorted {
                        if let Value::String(s) = script {
                            output.push_str(&format!("{} = '''\n{}\n'''\n", platform, s.trim()));
                        } else {
                            output.push_str(&format!(
                                "{} = {}\n",
                                platform,
                                serialize_value(script)?
                            ));
                        }
                    }
                }

                // Also include any other properties
                for (key, val) in table {
                    if key != "scripts" {
                        output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
                    }
                }
            } else {
                // Already flat structure or other properties
                let sorted: BTreeMap<_, _> = table.iter().collect();
                for (key, val) in sorted {
                    if let Value::String(s) = val {
                        // Assume string values in script installers are scripts
                        output.push_str(&format!("{} = '''\n{}\n'''\n", key, s.trim()));
                    } else {
                        output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
                    }
                }
            }
        } else {
            // Non-script installer - keep existing logic
            if let Some(scripts) = table.get("scripts") {
                // This shouldn't happen for non-script installers, but handle it
                output.push_str(&format!(
                    "[tools.{}.installers.{}]\n",
                    tool_name, installer_name
                ));
                for (key, val) in table {
                    if key != "scripts" {
                        output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
                    }
                }

                if let Value::Table(scripts_table) = scripts {
                    output.push('\n');
                    output.push_str(&format!(
                        "[tools.{}.installers.{}.scripts]\n",
                        tool_name, installer_name
                    ));
                    let sorted: BTreeMap<_, _> = scripts_table.iter().collect();
                    for (platform, script) in sorted {
                        if let Value::String(s) = script {
                            output.push_str(&format!("{} = '''\n{}\n'''\n", platform, s.trim()));
                        } else {
                            output.push_str(&format!(
                                "{} = {}\n",
                                platform,
                                serialize_value(script)?
                            ));
                        }
                    }
                }
            } else {
                // Simple installer config
                output.push_str(&format!(
                    "[tools.{}.installers.{}]\n",
                    tool_name, installer_name
                ));
                output.push_str(&serialize_table_contents(
                    table,
                    &["package", "repo", "pattern", "url"],
                )?);
            }
        }
    }

    Ok(output)
}

fn serialize_installer_table(table: &toml::map::Map<String, Value>) -> Result<String> {
    let mut output = String::new();

    // Define order for installer properties
    let priority_keys = [
        "type",
        "check",
        "install",
        "uninstall",
        "install_output_pattern",
        "version_check",
    ];

    // Write priority keys first in order
    for key in &priority_keys {
        if let Some(val) = table.get(*key) {
            match key {
                &"install_output_pattern" => {
                    // Use raw strings for patterns
                    if let Value::String(s) = val {
                        output.push_str(&format!("{} = '''{}'''\n", key, s));
                    } else {
                        output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
                    }
                }
                &"version_check" => {
                    // Keep as inline table
                    if let Value::Table(vc_table) = val {
                        output.push_str(&format!(
                            "{} = {}\n",
                            key,
                            serialize_inline_table(vc_table)?
                        ));
                    } else {
                        output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
                    }
                }
                _ => {
                    output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
                }
            }
        }
    }

    // Write any remaining keys
    let sorted: BTreeMap<_, _> = table.iter().collect();
    for (key, val) in sorted {
        if !priority_keys.contains(&key.as_str()) {
            output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
        }
    }

    Ok(output)
}

fn serialize_table_contents(
    table: &toml::map::Map<String, Value>,
    priority_keys: &[&str],
) -> Result<String> {
    let mut output = String::new();

    // Write priority keys first
    for key in priority_keys {
        if let Some(val) = table.get(*key) {
            output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
        }
    }

    // Write remaining keys alphabetically
    let sorted: BTreeMap<_, _> = table.iter().collect();
    for (key, val) in sorted {
        if !priority_keys.contains(&key.as_str()) {
            output.push_str(&format!("{} = {}\n", key, serialize_value(val)?));
        }
    }

    Ok(output)
}

fn serialize_inline_table(table: &toml::map::Map<String, Value>) -> Result<String> {
    let mut parts = Vec::new();

    // Define order for version_check
    let priority_keys = ["method", "command", "url", "path"];

    for key in &priority_keys {
        if let Some(val) = table.get(*key) {
            parts.push(format!("{} = {}", key, serialize_value(val)?));
        }
    }

    // Add any remaining keys
    for (key, val) in table {
        if !priority_keys.contains(&key.as_str()) {
            parts.push(format!("{} = {}", key, serialize_value(val)?));
        }
    }

    Ok(format!("{{ {} }}", parts.join(", ")))
}

fn serialize_value(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => {
            // Check if string contains special characters that need escaping
            if s.contains('"') || s.contains('\\') || s.contains('\n') {
                Ok(format!("\"{}\"", escape_string(s)))
            } else {
                Ok(format!("\"{}\"", s))
            }
        }
        Value::Integer(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        Value::Array(arr) => {
            let items: Result<Vec<String>> = arr.iter().map(serialize_value).collect();
            Ok(format!("[{}]", items?.join(", ")))
        }
        Value::Table(t) => {
            // This shouldn't happen in our use case
            serialize_inline_table(t)
        }
        Value::Datetime(dt) => Ok(format!("\"{}\"", dt)),
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
