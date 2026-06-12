//! Forge (code-hosting) types shared between core and engine.

use serde::{Deserialize, Serialize};

/// The CI status for a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatus {
    Pending,
    Passing,
    Failing,
    Unknown,
}

/// A single comment on a pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrComment {
    pub id: String,
    pub body: String,
    pub author: String,
}

/// The result of polling a pull request for status, approvals, and comments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrPollResult {
    pub ci_status: CiStatus,
    pub approved: bool,
    pub comments: Vec<PrComment>,
}
