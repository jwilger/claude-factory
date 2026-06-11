//! Slice kind — mirrors the emc closed enum for slice types.

use serde::{Deserialize, Serialize};

/// The kind of a vertical slice, as defined by emc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SliceKind {
    StateChange,
    StateView,
    Translation,
    Automation,
}

impl SliceKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StateChange => "state_change",
            Self::StateView => "state_view",
            Self::Translation => "translation",
            Self::Automation => "automation",
        }
    }
}

impl std::fmt::Display for SliceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
