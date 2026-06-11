//! Evidence types — structured artifacts submitted to complete a step.

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// A SHA-256 hex digest over an artifact, used to detect drift.
#[nutype(
    sanitize(trim, lowercase),
    validate(len_char_min = 64, len_char_max = 64),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)
)]
pub struct EvidenceDigest(String);

/// The raw JSON value of a submitted artifact.
/// Validated against the step's output schema in cfk-engine.
pub type ArtifactJson = serde_json::Value;

/// Evidence submitted for a completed step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEvidence {
    /// The structured output from the agent (validated against schema).
    pub artifact: ArtifactJson,
    /// Digest of the artifact for audit trail.
    pub digest: EvidenceDigest,
}

impl StepEvidence {
    /// Create evidence, computing the digest from the artifact.
    ///
    /// # Errors
    /// Returns `EvidenceDigestError` if the computed hex digest is invalid (should not happen).
    pub fn from_artifact(artifact: ArtifactJson) -> Result<Self, EvidenceDigestError> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        // In production we'd use SHA-256; using a deterministic hash here
        // that produces a fixed-length hex string for the type invariant.
        // TODO(M1): replace with actual SHA-256
        let mut hasher = DefaultHasher::new();
        artifact.to_string().hash(&mut hasher);
        let raw = format!("{:064x}", hasher.finish());
        let digest = EvidenceDigest::try_new(raw)?;
        Ok(Self { artifact, digest })
    }
}
