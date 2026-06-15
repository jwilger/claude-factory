//! Event store paths for the `eventcore-fs` persistent store.
//!
//! The kernel is event-sourced. Every state change produces an event written
//! to the `eventcore-fs` store at `.claude-factory/eventstore/`. The store is
//! git-tracked (union merge on `events/**`) and is the authoritative record.
//! In-memory projection is rebuilt by replaying the store on startup.

use std::path::PathBuf;

/// Path to the git-tracked JSON event export directory in a product repo.
#[must_use]
pub fn event_export_dir(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".claude-factory").join("events").join("v1")
}

/// Root directory for the `eventcore-fs` v2 event store.
///
/// `eventcore-fs` manages subdirectory layout internally under this root.
#[must_use]
pub fn eventcore_store_dir(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".claude-factory").join("eventstore")
}

/// Path to the `SQLite` operational cache for a project.
///
/// Lives outside the repo (XDG state dir) so it is never accidentally committed.
#[must_use]
pub fn sqlite_cache_path(project_root: &std::path::Path) -> PathBuf {
    let state_dir = std::env::var("XDG_STATE_HOME").map_or_else(
        |_| {
            std::env::var("HOME").map_or_else(
                |_| PathBuf::from("/tmp"),
                PathBuf::from,
            )
            .join(".local")
            .join("state")
        },
        PathBuf::from,
    );
    let hash = sha256_path(project_root);
    state_dir
        .join("claude-factory")
        .join("projects")
        .join(hash)
        .join("events.sqlite3")
}

fn sha256_path(path: &std::path::Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("{:016x}", h.finish())
}
