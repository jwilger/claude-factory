//! Gate verdict types — the result of an independent review gate.

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// Which review gate a verdict belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    TestReview,
    ImplementationReview,
}

/// A human-readable explanation of why a gate was vetoed.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct VetoReason(String);

/// The verdict from a review gate (test review or implementation review).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum GateVerdict {
    Approved,
    Vetoed { reason: VetoReason },
}

impl GateVerdict {
    #[must_use]
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved)
    }

    #[must_use]
    pub fn is_vetoed(&self) -> bool {
        matches!(self, Self::Vetoed { .. })
    }
}
