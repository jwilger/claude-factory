//! Check configuration — loaded from `.claude-factory/checks.toml`.
//!
//! Each entry maps a check name to the shell command the kernel runs.
//! If `checks.toml` does not exist, built-in defaults are used.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// One configured check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Shell command to execute (run via `sh -c`).
    pub command: String,
}

/// The full set of checks for a project.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChecksConfig {
    #[serde(default)]
    pub checks: HashMap<String, CheckConfig>,
}

impl ChecksConfig {
    /// Return the command for `check_name`, falling back to built-in defaults.
    #[must_use]
    pub fn command_for(&self, check_name: &str) -> Option<String> {
        if let Some(cfg) = self.checks.get(check_name) {
            return Some(cfg.command.clone());
        }
        // Built-in defaults
        match check_name {
            "tests" => Some("cargo nextest run".to_string()),
            "lint" => Some("cargo clippy -- -D warnings".to_string()),
            "build" => Some("cargo build".to_string()),
            _ => None,
        }
    }
}

/// Load checks config from `<project_root>/.claude-factory/checks.toml`.
///
/// Returns `ChecksConfig::default()` (empty — built-ins apply) if the file
/// does not exist.
///
/// # Errors
/// Returns an error if the file exists but cannot be parsed.
pub fn load_checks(project_root: &Path) -> anyhow::Result<ChecksConfig> {
    let path = project_root.join(".claude-factory").join("checks.toml");
    if !path.exists() {
        return Ok(ChecksConfig::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let cfg: ChecksConfig = toml::from_str(&content)?;
    Ok(cfg)
}
