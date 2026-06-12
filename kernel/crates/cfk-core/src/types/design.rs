//! Design-system phase types — Atomic Design component inventory.

use crate::types::ids::ComponentId;
use nutype::nutype;
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

/// Human-readable name for a design-system component.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ComponentName(String);

/// A design component in the Atomic Design inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignComponent {
    pub id: ComponentId,
    pub name: ComponentName,
    pub kind: AtomicKind,
    /// Optional reference to the emc slice this component satisfies.
    pub slice_ref: Option<String>,
}
