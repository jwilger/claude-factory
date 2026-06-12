//! Review-phase slice state machine — pure types and constructors.

use crate::types::ids::WorkItemId;
use serde::{Deserialize, Serialize};

/// The current phase of a review slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSlicePhase {
    /// No PR exists yet; kernel will ask conductor to open one.
    WaitingForPr,
    /// PR is open; kernel will ask conductor to poll for CI/reviews/comments.
    PrOpen,
    /// New comments arrived; pending triage work items must be completed first.
    CommentTriagePending,
    /// All checks passed and the PR is approved; kernel will ask conductor to merge.
    AllGreen,
    /// PR merged; slice is done.
    Merged,
}

/// Runtime state for a review slice work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSliceState {
    pub work_item_id: WorkItemId,
    pub phase: ReviewSlicePhase,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
    /// Comment IDs already seen, to detect new comments on poll.
    pub seen_comment_ids: Vec<String>,
    /// `(comment_id, triage_work_item_id)` for each unsettled comment.
    pub pending_triage: Vec<(String, WorkItemId)>,
}

impl ReviewSliceState {
    #[must_use]
    pub fn new(work_item_id: WorkItemId) -> Self {
        Self {
            work_item_id,
            phase: ReviewSlicePhase::WaitingForPr,
            pr_number: None,
            pr_url: None,
            seen_comment_ids: Vec::new(),
            pending_triage: Vec::new(),
        }
    }
}
