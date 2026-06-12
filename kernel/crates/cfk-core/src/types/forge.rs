//! Forge (code-hosting) types shared between core and engine.

use nutype::nutype;
use serde::{Deserialize, Serialize};

/// A pull-request URL from the code-hosting forge.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct PrUrl(String);

/// The forge-assigned numeric identifier for a pull request.
#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize))]
pub struct PrNumber(u64);

/// The forge-assigned string identifier for a pull-request comment.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct CommentId(String);

/// The text body of a pull-request comment.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct CommentBody(String);

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
    pub id: CommentId,
    pub body: CommentBody,
    pub author: String,
}

/// The result of polling a pull request for status, approvals, and comments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrPollResult {
    pub ci_status: CiStatus,
    pub approved: bool,
    pub comments: Vec<PrComment>,
}
