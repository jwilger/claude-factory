//! Architecture phase imperative shell — ARCHITECTURE.md projection.
//!
//! The ARCHITECTURE.md file is kernel-rendered from the accepted ADR registry.
//! It is never LLM-written; the kernel projects it deterministically after each
//! ADR is accepted.

use cfk_core::types::architecture::{AdrRecord, AdrStatus};
use std::fmt::Write as _;
use std::path::Path;
use thiserror::Error;

/// Error returned when ARCHITECTURE.md cannot be written.
#[derive(Debug, Error)]
#[error("failed to write ARCHITECTURE.md: {0}")]
pub struct ArchitectureError(#[from] std::io::Error);

/// Render and write ARCHITECTURE.md to the product repo root.
///
/// # Errors
/// Returns an error if the file cannot be written.
pub fn project_architecture_md(root: &Path, adrs: &[AdrRecord]) -> Result<(), ArchitectureError> {
    let accepted: Vec<_> = adrs
        .iter()
        .filter(|r| r.status == AdrStatus::Accepted)
        .collect();

    let mut out = String::from(
        "# Architecture\n\n\
         This document is automatically projected by Claude-Factory from the\n\
         accepted Architecture Decision Records. Do not edit by hand.\n\n",
    );

    if accepted.is_empty() {
        out.push_str("_No architecture decisions recorded yet._\n");
    } else {
        out.push_str("## Accepted Decisions\n\n");
        for (i, adr) in accepted.iter().enumerate() {
            let n = i + 1;
            write!(out, "### ADR-{n:04}: {}\n\n", adr.title).expect("write to String never fails");
            out.push_str(&adr.content);
            if !adr.content.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }

    let path = root.join("ARCHITECTURE.md");
    std::fs::write(path, out)?;
    Ok(())
}
