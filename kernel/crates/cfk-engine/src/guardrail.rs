//! Imperative shell for the product-source edit guardrail.
//!
//! Gathers the facts the pure policy ([`cfk_core::guardrail::decide`]) needs —
//! is this a factory project, is the operator bypass present, does the path
//! match the project's protected globs, does the editing session hold an active
//! lease — and returns the verdict. All I/O (filesystem checks, config read,
//! event replay) lives here; the decision itself is pure.

use std::path::Path;

use chrono::{DateTime, Utc};
use cfk_core::{
    guardrail::{GuardrailDecision, GuardrailFacts, decide},
    types::lease::SessionIdentity,
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use thiserror::Error;

use crate::{events::EventStoreError, loader::load_project_state};

/// The Claude-Factory state directory at a project root.
const FACTORY_DIR: &str = ".claude-factory";
/// Operator bypass sentinel, relative to the factory dir.
const BYPASS_FILE: &str = "LEASE_BYPASS";
/// Per-project guardrail config, relative to the factory dir.
const CONFIG_FILE: &str = "guardrail.json";
/// Default protected globs when no config is present — any `src/` tree.
const DEFAULT_PROTECTED_GLOBS: &[&str] = &["**/src/**"];

/// Errors gathering guardrail facts. Callers should treat any error as a reason
/// to block (fail closed): a guardrail that cannot evaluate must not permit.
#[derive(Debug, Error)]
pub enum GuardrailError {
    #[error("failed to read guardrail config {path}: {source}")]
    ConfigRead {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse guardrail config {path}: {source}")]
    ConfigParse {
        path: String,
        source: serde_json::Error,
    },
    #[error("invalid protected glob {pattern:?}: {source}")]
    InvalidGlob {
        pattern: String,
        source: globset::Error,
    },
    #[error("failed to load project state: {0}")]
    State(#[from] EventStoreError),
}

/// On-disk shape of `.claude-factory/guardrail.json`.
#[derive(Debug, Deserialize)]
struct GuardrailConfigFile {
    protected_globs: Vec<String>,
}

/// The compiled set of globs that mark a path as gated product source.
pub struct ProtectedGlobs {
    set: GlobSet,
}

impl ProtectedGlobs {
    /// Compile a set of glob patterns.
    fn new(patterns: &[String]) -> Result<Self, GuardrailError> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern).map_err(|source| GuardrailError::InvalidGlob {
                pattern: pattern.clone(),
                source,
            })?;
            builder.add(glob);
        }
        let set = builder.build().map_err(|source| GuardrailError::InvalidGlob {
            pattern: patterns.join(", "),
            source,
        })?;
        Ok(Self { set })
    }

    /// The built-in defaults, used when a project declares no config.
    fn defaults() -> Result<Self, GuardrailError> {
        let patterns: Vec<String> = DEFAULT_PROTECTED_GLOBS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        Self::new(&patterns)
    }

    /// True when `relative_path` matches any protected glob.
    #[must_use]
    pub fn matches(&self, relative_path: &Path) -> bool {
        self.set.is_match(relative_path)
    }
}

/// Load the project's protected globs from `<factory_dir>/guardrail.json`,
/// falling back to [`ProtectedGlobs::defaults`] when the file is absent.
fn load_protected_globs(factory_dir: &Path) -> Result<ProtectedGlobs, GuardrailError> {
    let config_path = factory_dir.join(CONFIG_FILE);
    if !config_path.exists() {
        return ProtectedGlobs::defaults();
    }
    let raw = std::fs::read_to_string(&config_path).map_err(|source| GuardrailError::ConfigRead {
        path: config_path.display().to_string(),
        source,
    })?;
    let config: GuardrailConfigFile =
        serde_json::from_str(&raw).map_err(|source| GuardrailError::ConfigParse {
            path: config_path.display().to_string(),
            source,
        })?;
    ProtectedGlobs::new(&config.protected_globs)
}

/// Decide whether `session` may edit `edited_file` in the project at
/// `project_root`, as of `now`.
///
/// `edited_file` may be absolute or relative to `project_root`; protected-glob
/// matching is performed on the path relative to the root.
///
/// # Errors
/// Returns a [`GuardrailError`] if the project's guardrail config cannot be read
/// or parsed, a configured glob is invalid, or the event store cannot be loaded.
/// Callers should treat any error as a reason to block (fail closed).
pub fn check_guardrail(
    project_root: &Path,
    edited_file: &Path,
    session: &SessionIdentity,
    now: DateTime<Utc>,
) -> Result<GuardrailDecision, GuardrailError> {
    let factory_dir = project_root.join(FACTORY_DIR);

    // Not a factory project → nothing to guard.
    if !factory_dir.is_dir() {
        return Ok(decide(GuardrailFacts {
            is_factory_project: false,
            bypass_active: false,
            path_is_protected: false,
            session_holds_active_lease: false,
        }));
    }

    // Operator bypass short-circuits before any config/state load, so a bypassed
    // project need not carry valid guardrail config.
    if factory_dir.join(BYPASS_FILE).exists() {
        return Ok(decide(GuardrailFacts {
            is_factory_project: true,
            bypass_active: true,
            path_is_protected: false,
            session_holds_active_lease: false,
        }));
    }

    let globs = load_protected_globs(&factory_dir)?;
    let relative = edited_file.strip_prefix(project_root).unwrap_or(edited_file);
    let path_is_protected = globs.matches(relative);

    let session_holds_active_lease = match load_project_state(project_root)? {
        Some(state) => state
            .leases
            .iter()
            .any(|lease| &lease.session_identity == session && !lease.is_expired(now)),
        None => false,
    };

    Ok(decide(GuardrailFacts {
        is_factory_project: true,
        bypass_active: false,
        path_is_protected,
        session_holds_active_lease,
    }))
}
