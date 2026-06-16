//! Pure per-slice promotion-chain logic (ADR 0011).
//!
//! Each verified emc slice flows through a fixed chain of phases:
//! Architecture (triage) → `DesignSystem` (triage) → Development. This module
//! decides, from the work items that already exist for one slice, which single
//! next item — if any — should be spawned to advance that slice's chain. It is
//! pure: the imperative shell reads the verified model, groups work items by
//! slug, calls this function per slice, and emits the resulting `WorkItemAdded`
//! events.

use crate::{
    state_machine::work_item::WorkItemStatus,
    types::{phase::PhaseKind, routing::WorkType},
};

/// The next work item to spawn to advance a slice's promotion chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainHead {
    pub phase: PhaseKind,
    pub work_type: WorkType,
}

/// Decide the single next work item to spawn for one emc slice, given the
/// `(phase, status)` of every work item that already exists for that slice's
/// slug.
///
/// The chain is Architecture (triage) → `DesignSystem` (triage) → Development.
/// A phase is "satisfied" only when at least one item exists for it and every
/// such item is terminal (`Done`/`Abandoned`) with at least one `Done` — so an
/// architecture triage that spawned an ADR sub-item does not advance the chain
/// until that ADR, too, is complete.
///
/// `dev_work_type` is the Development work type chosen for this slice (e.g.
/// `MechanicalTransform` vs `NarrowestStepImplementation`), supplied by the
/// caller so this function stays free of emc concerns.
///
/// Returns `None` when the chain is already as far along as the existing items
/// allow — nothing to spawn yet.
#[must_use]
pub fn next_slice_promotion(
    existing: &[(PhaseKind, WorkItemStatus)],
    dev_work_type: WorkType,
) -> Option<ChainHead> {
    if existing.is_empty() {
        return Some(ChainHead {
            phase: PhaseKind::Architecture,
            work_type: WorkType::ArchitectureTriage,
        });
    }

    let phase_satisfied = |phase: PhaseKind| {
        let statuses: Vec<WorkItemStatus> =
            existing.iter().filter(|(p, _)| *p == phase).map(|(_, s)| *s).collect();
        !statuses.is_empty()
            && statuses.contains(&WorkItemStatus::Done)
            && statuses
                .iter()
                .all(|s| matches!(s, WorkItemStatus::Done | WorkItemStatus::Abandoned))
    };
    let phase_present = |phase: PhaseKind| existing.iter().any(|(p, _)| *p == phase);

    if phase_satisfied(PhaseKind::Architecture) && !phase_present(PhaseKind::DesignSystem) {
        return Some(ChainHead {
            phase: PhaseKind::DesignSystem,
            work_type: WorkType::DesignTriage,
        });
    }
    if phase_satisfied(PhaseKind::DesignSystem) && !phase_present(PhaseKind::Development) {
        return Some(ChainHead { phase: PhaseKind::Development, work_type: dev_work_type });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{next_slice_promotion, ChainHead};
    use crate::{
        state_machine::work_item::WorkItemStatus,
        types::{phase::PhaseKind, routing::WorkType},
    };

    const DEV: WorkType = WorkType::NarrowestStepImplementation;

    #[test]
    fn no_items_starts_chain_at_architecture_triage() {
        assert_eq!(
            next_slice_promotion(&[], DEV),
            Some(ChainHead {
                phase: PhaseKind::Architecture,
                work_type: WorkType::ArchitectureTriage,
            })
        );
    }

    #[test]
    fn architecture_in_progress_does_not_advance() {
        let existing = [(PhaseKind::Architecture, WorkItemStatus::InProgress)];
        assert_eq!(next_slice_promotion(&existing, DEV), None);
    }

    #[test]
    fn architecture_done_spawns_design_triage() {
        let existing = [(PhaseKind::Architecture, WorkItemStatus::Done)];
        assert_eq!(
            next_slice_promotion(&existing, DEV),
            Some(ChainHead { phase: PhaseKind::DesignSystem, work_type: WorkType::DesignTriage })
        );
    }

    #[test]
    fn architecture_with_pending_adr_sub_item_does_not_advance() {
        // Triage completed but the ADR it spawned is still in progress.
        let existing = [
            (PhaseKind::Architecture, WorkItemStatus::Done),
            (PhaseKind::Architecture, WorkItemStatus::InProgress),
        ];
        assert_eq!(next_slice_promotion(&existing, DEV), None);
    }

    #[test]
    fn design_done_spawns_development_with_given_work_type() {
        let existing = [
            (PhaseKind::Architecture, WorkItemStatus::Done),
            (PhaseKind::DesignSystem, WorkItemStatus::Done),
        ];
        assert_eq!(
            next_slice_promotion(&existing, WorkType::MechanicalTransform),
            Some(ChainHead {
                phase: PhaseKind::Development,
                work_type: WorkType::MechanicalTransform,
            })
        );
    }

    #[test]
    fn design_in_progress_does_not_advance() {
        let existing = [
            (PhaseKind::Architecture, WorkItemStatus::Done),
            (PhaseKind::DesignSystem, WorkItemStatus::InProgress),
        ];
        assert_eq!(next_slice_promotion(&existing, DEV), None);
    }

    #[test]
    fn full_chain_present_spawns_nothing() {
        let existing = [
            (PhaseKind::Architecture, WorkItemStatus::Done),
            (PhaseKind::DesignSystem, WorkItemStatus::Done),
            (PhaseKind::Development, WorkItemStatus::Ready),
        ];
        assert_eq!(next_slice_promotion(&existing, DEV), None);
    }

    #[test]
    fn abandoned_architecture_alongside_done_still_advances() {
        let existing = [
            (PhaseKind::Architecture, WorkItemStatus::Abandoned),
            (PhaseKind::Architecture, WorkItemStatus::Done),
        ];
        assert_eq!(
            next_slice_promotion(&existing, DEV),
            Some(ChainHead { phase: PhaseKind::DesignSystem, work_type: WorkType::DesignTriage })
        );
    }
}
