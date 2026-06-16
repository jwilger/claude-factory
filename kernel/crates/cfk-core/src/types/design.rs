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

/// Which layer owns a design component (ADR 0012).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentOwnership {
    /// Reusable across slices — lives in the platform UI library.
    Platform,
    /// Bespoke to one vertical slice — owned by that slice. Default when an
    /// older event or caller did not classify the component, since keeping
    /// unclassified work slice-local avoids polluting the shared library.
    #[default]
    Slice,
}

/// A design component in the Atomic Design inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignComponent {
    pub id: ComponentId,
    pub name: ComponentName,
    pub kind: AtomicKind,
    /// Which layer owns this component (platform UI library vs the slice).
    #[serde(default)]
    pub ownership: ComponentOwnership,
    /// Optional reference to the emc slice this component satisfies.
    pub slice_ref: Option<String>,
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test uses unwrap for assertion clarity")]
mod tests {
    use super::{AtomicKind, ComponentName, ComponentOwnership, DesignComponent};
    use crate::types::ids::ComponentId;

    fn sample(ownership: ComponentOwnership) -> DesignComponent {
        DesignComponent {
            id: ComponentId::new(),
            name: ComponentName::try_new("PrimaryButton".to_string()).unwrap(),
            kind: AtomicKind::Atom,
            ownership,
            slice_ref: None,
        }
    }

    #[test]
    fn ownership_round_trips() {
        let c = sample(ComponentOwnership::Platform);
        let back: DesignComponent =
            serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
        assert_eq!(back.ownership, ComponentOwnership::Platform);
    }

    #[test]
    fn ownership_defaults_to_slice_for_legacy_events() {
        // A component/event written before `ownership` existed must still load.
        let mut v = serde_json::to_value(sample(ComponentOwnership::Platform)).unwrap();
        v.as_object_mut().unwrap().remove("ownership");
        let legacy: DesignComponent = serde_json::from_value(v).unwrap();
        assert_eq!(legacy.ownership, ComponentOwnership::Slice);
    }
}
