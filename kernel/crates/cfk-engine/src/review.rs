//! Review-phase command handlers — the imperative shell for the review state machine.
//!
//! Each handler takes the current `ProjectState`, calls the forge adapter, and
//! returns the events to emit. No business logic — all state transitions are in
//! `cfk-core::state_machine::review`.

use crate::{
    events::FactoryEvent,
    forge::{ForgeAdapter, PrSpec},
    project::ProjectState,
};
use cfk_core::{
    state_machine::review::ReviewSlicePhase,
    types::ids::WorkItemId,
};
use std::sync::Arc;
use thiserror::Error;

/// Errors from review command handlers.
#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("work item {0:?} not found")]
    NotFound(WorkItemId),

    #[error("work item {0:?} has no active review state")]
    NoReviewState(WorkItemId),

    #[error("review slice is in an unexpected phase: {0:?}")]
    UnexpectedPhase(ReviewSlicePhase),

    #[error("forge error: {0}")]
    Forge(#[from] anyhow::Error),
}

/// Handle `cf_pr_open` — open a PR on the forge and record `ReviewSliceStarted`.
///
/// # Errors
/// Returns `ReviewError` if the work item is missing, already has a PR, or the
/// forge call fails.
pub async fn handle_pr_open(
    state: &ProjectState,
    work_item_id: &WorkItemId,
    title: String,
    body: String,
    head: String,
    base: String,
    forge: &Arc<dyn ForgeAdapter>,
) -> Result<Vec<FactoryEvent>, ReviewError> {
    let item = state.work_items.iter().find(|i| &i.id == work_item_id)
        .ok_or_else(|| ReviewError::NotFound(work_item_id.clone()))?;

    if state.review_states.get(work_item_id)
        .is_some_and(|r| r.phase != ReviewSlicePhase::WaitingForPr)
    {
        return Err(ReviewError::UnexpectedPhase(
            state.review_states[work_item_id].phase.clone()
        ));
    }

    let spec = PrSpec { title, body, head, base };
    let opened = forge.open_pr(&spec).await?;

    Ok(vec![FactoryEvent::ReviewSliceStarted {
        work_item_id: item.id.clone(),
        pr_number: opened.number,
        pr_url: opened.url,
    }])
}

/// Handle `cf_pr_poll` — poll the forge for CI, reviews, and new comments.
///
/// Returns events to emit: possibly `ReviewCommentTriageCreated` items for new
/// comments, and/or `ReviewAllGreen` if checks pass and PR is approved.
///
/// # Errors
/// Returns `ReviewError` if the work item or its review state is missing, or the
/// forge call fails.
pub async fn handle_pr_poll(
    state: &ProjectState,
    work_item_id: &WorkItemId,
    forge: &Arc<dyn ForgeAdapter>,
) -> Result<Vec<FactoryEvent>, ReviewError> {
    let review = state.review_states.get(work_item_id)
        .ok_or_else(|| ReviewError::NoReviewState(work_item_id.clone()))?;

    if review.phase != ReviewSlicePhase::PrOpen {
        return Err(ReviewError::UnexpectedPhase(review.phase.clone()));
    }

    let pr_number = review.pr_number
        .ok_or_else(|| ReviewError::NoReviewState(work_item_id.clone()))?;

    let poll = forge.poll_pr(pr_number).await?;

    let mut events = Vec::new();

    // Detect new comments not yet seen.
    for comment in &poll.comments {
        if !review.seen_comment_ids.contains(&comment.id.to_string()) {
            let triage_item_id = cfk_core::types::ids::WorkItemId::new();
            events.push(FactoryEvent::ReviewCommentTriageCreated {
                review_work_item_id: work_item_id.clone(),
                triage_item_id,
                comment_id: comment.id.to_string(),
                comment_body: comment.body.to_string(),
            });
        }
    }

    // If no new comments and CI is green + approved, emit AllGreen.
    if events.is_empty()
        && poll.approved
        && poll.ci_status == cfk_core::types::forge::CiStatus::Passing
    {
        events.push(FactoryEvent::ReviewAllGreen { work_item_id: work_item_id.clone() });
    }

    Ok(events)
}

/// Handle `cf_pr_merge` — merge the PR and record `ReviewPrMerged`.
///
/// # Errors
/// Returns `ReviewError` if the work item is not in `AllGreen` phase or the
/// forge call fails.
pub async fn handle_pr_merge(
    state: &ProjectState,
    work_item_id: &WorkItemId,
    forge: &Arc<dyn ForgeAdapter>,
) -> Result<Vec<FactoryEvent>, ReviewError> {
    let review = state.review_states.get(work_item_id)
        .ok_or_else(|| ReviewError::NoReviewState(work_item_id.clone()))?;

    if review.phase != ReviewSlicePhase::AllGreen {
        return Err(ReviewError::UnexpectedPhase(review.phase.clone()));
    }

    let pr_number = review.pr_number
        .ok_or_else(|| ReviewError::NoReviewState(work_item_id.clone()))?;

    forge.merge_pr(pr_number).await?;

    Ok(vec![
        FactoryEvent::ReviewPrMerged { work_item_id: work_item_id.clone() },
        FactoryEvent::WorkItemCompleted { work_item_id: work_item_id.clone() },
    ])
}
