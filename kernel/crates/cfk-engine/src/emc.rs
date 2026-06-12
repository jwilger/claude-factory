//! Read emc model events and extract verified slice definitions.
//!
//! The kernel reads `model/events/v1/*.json` directly (no LLM in the loop).
//! Only slices from workflows that have a `WorkflowReadinessDeclared` event are returned.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Error returned when emc model events cannot be read.
#[derive(Debug, Error)]
pub enum EmcError {
    #[error("failed to read emc events directory {dir}: {source}")]
    ReadDir { dir: PathBuf, #[source] source: io::Error },
    #[error("failed to read emc event file {path}: {source}")]
    ReadFile { path: PathBuf, #[source] source: io::Error },
    #[error("failed to parse emc event file {path}: {source}")]
    ParseFile { path: PathBuf, #[source] source: serde_json::Error },
}

/// A slice extracted from a formally-verified emc workflow model.
#[derive(Debug, Clone)]
pub struct EmcSlice {
    pub workflow: String,
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub description: String,
}

#[derive(Deserialize)]
struct EmcEvent {
    #[serde(rename = "type")]
    event_type: String,
    payload: serde_json::Value,
}

/// Read all emc model events under `project_root/model/events/v1/` and return
/// every `SliceAdded` entry whose workflow has a `WorkflowReadinessDeclared` event.
///
/// # Errors
/// Returns an error if the model directory cannot be read or any JSON file fails to parse.
pub fn read_verified_slices(project_root: &Path) -> Result<Vec<EmcSlice>, EmcError> {
    let events_dir = project_root.join("model").join("events").join("v1");
    if !events_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&events_dir)
        .map_err(|source| EmcError::ReadDir { dir: events_dir.clone(), source })?
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    let mut events: Vec<EmcEvent> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let path = entry.path();
        let raw = std::fs::read_to_string(&path)
            .map_err(|source| EmcError::ReadFile { path: path.clone(), source })?;
        let ev: EmcEvent = serde_json::from_str(&raw)
            .map_err(|source| EmcError::ParseFile { path, source })?;
        events.push(ev);
    }

    let verified: HashSet<String> = events
        .iter()
        .filter(|e| e.event_type == "WorkflowReadinessDeclared")
        .filter_map(|e| e.payload.get("workflow").and_then(|v| v.as_str()).map(String::from))
        .collect();

    let slices = events
        .iter()
        .filter(|e| e.event_type == "SliceAdded")
        .filter_map(|e| {
            let p = &e.payload;
            let workflow = p.get("workflow")?.as_str()?.to_owned();
            if !verified.contains(&workflow) {
                return None;
            }
            Some(EmcSlice {
                workflow,
                slug: p.get("slug")?.as_str()?.to_owned(),
                name: p.get("name")?.as_str()?.to_owned(),
                kind: p.get("kind")?.as_str()?.to_owned(),
                description: p.get("description")?.as_str()?.to_owned(),
            })
        })
        .collect();

    Ok(slices)
}
