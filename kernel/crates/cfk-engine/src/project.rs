//! In-memory project state for the running kernel.
//!
//! The kernel loads all events from the store on startup and projects them
//! into `ProjectState`. All commands read/write through this projection.
//! The projection is rebuilt from scratch on restart (event sourcing).

use cfk_core::{
    state_machine::{
        architecture::AdrWorkItemState,
        design::DesignWorkItemState,
        discovery::DiscoveryState,
        review::ReviewSliceState,
        work_item::{WorkItem, WorkItemStatus},
    },
    types::{
        architecture::AdrRecord,
        design::DesignComponent,
        ids::{ProjectId, WorkItemId},
        lease::Lease,
        metrics::WorkTypeMetrics,
        phase::PhaseKind,
        routing::{RoutingTable, WorkType},
        tdd::DevSliceState,
    },
};
use std::collections::HashMap;
use std::path::PathBuf;


/// Runtime projection of all factory state for one product project.
#[derive(Debug)]
pub struct ProjectState {
    pub id: ProjectId,
    pub root: PathBuf,
    pub routing: RoutingTable,
    pub work_items: Vec<WorkItem>,
    pub leases: Vec<Lease>,
    /// TDD state for each in-progress development slice.
    pub dev_states: HashMap<WorkItemId, DevSliceState>,
    /// Review state for each in-progress review slice.
    pub review_states: HashMap<WorkItemId, ReviewSliceState>,
    /// Discovery state for each in-progress discovery work item.
    pub discovery_states: HashMap<WorkItemId, DiscoveryState>,
    /// ADR state for each in-progress architecture work item.
    pub adr_states: HashMap<WorkItemId, AdrWorkItemState>,
    /// Global registry of all ADRs (proposed, accepted, rejected).
    pub adrs: Vec<AdrRecord>,
    /// Design-system state for each in-progress design work item.
    pub design_states: HashMap<WorkItemId, DesignWorkItemState>,
    /// Global Atomic Design component inventory.
    pub design_inventory: Vec<DesignComponent>,
    /// Which phases are active (initialized).
    pub active_phases: Vec<PhaseKind>,
    /// Accumulated per-work-type metrics (veto rates, token costs).
    pub metrics: HashMap<WorkType, WorkTypeMetrics>,
}

impl ProjectState {
    /// Create a fresh project state for a newly initialized product repo.
    #[must_use]
    pub fn new(id: ProjectId, root: PathBuf, routing: RoutingTable) -> Self {
        Self {
            id,
            root,
            routing,
            work_items: Vec::new(),
            leases: Vec::new(),
            dev_states: HashMap::new(),
            review_states: HashMap::new(),
            discovery_states: HashMap::new(),
            adr_states: HashMap::new(),
            adrs: Vec::new(),
            design_states: HashMap::new(),
            design_inventory: Vec::new(),
            active_phases: PhaseKind::all().to_vec(),
            metrics: HashMap::new(),
        }
    }

    /// Return the phase WIP counts for the status dashboard.
    #[must_use]
    pub fn phase_counts(&self) -> HashMap<PhaseKind, PhaseCounts> {
        let mut counts: HashMap<PhaseKind, PhaseCounts> = HashMap::new();
        for item in &self.work_items {
            let c = counts.entry(item.phase).or_default();
            match item.status {
                WorkItemStatus::Ready => c.ready += 1,
                WorkItemStatus::InProgress => c.in_progress += 1,
                WorkItemStatus::Blocked => c.blocked += 1,
                WorkItemStatus::Done => c.done += 1,
                WorkItemStatus::Abandoned => {}
            }
        }
        counts
    }
}

/// Counts of work items by status for one phase.
#[derive(Debug, Default, Clone, Copy)]
pub struct PhaseCounts {
    pub ready: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub done: usize,
}
