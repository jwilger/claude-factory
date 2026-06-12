//! Design-system phase state machine — pure types.
//!
//! Each design-system work item represents a single Atomic Design component
//! to be built out by the design-system agent.

use crate::types::design::ComponentName;
use serde::{Deserialize, Serialize};

/// Phases of a design-system work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignPhase {
    /// Agent is building out the component.
    Building,
    /// Component is complete and in the inventory.
    Done,
}

/// Runtime state for an in-progress design-system work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignWorkItemState {
    pub phase: DesignPhase,
    pub component_name: Option<ComponentName>,
}

impl DesignWorkItemState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: DesignPhase::Building,
            component_name: None,
        }
    }
}

impl Default for DesignWorkItemState {
    fn default() -> Self {
        Self::new()
    }
}
