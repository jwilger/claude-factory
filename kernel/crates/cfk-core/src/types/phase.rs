//! Phase kinds — the six overlapping phases of the factory process.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhaseKind {
    Discovery,
    EventModeling,
    Architecture,
    DesignSystem,
    Development,
    Review,
}

impl PhaseKind {
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Discovery,
            Self::EventModeling,
            Self::Architecture,
            Self::DesignSystem,
            Self::Development,
            Self::Review,
        ]
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::EventModeling => "event-modeling",
            Self::Architecture => "architecture",
            Self::DesignSystem => "design-system",
            Self::Development => "development",
            Self::Review => "review",
        }
    }
}

impl std::fmt::Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
