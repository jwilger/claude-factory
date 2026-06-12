//! Design-system phase types — Atomic Design component inventory.

use crate::types::ids::ComponentId;
use serde::{Deserialize, Serialize};

/// Atomic Design level for a design component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomicKind {
    Quark,
    Atom,
    Molecule,
    Organism,
    Template,
    Page,
}

/// A design component in the Atomic Design inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignComponent {
    pub id: ComponentId,
    pub name: String,
    pub kind: AtomicKind,
    /// Optional reference to the emc slice this component satisfies.
    pub slice_ref: Option<String>,
}
