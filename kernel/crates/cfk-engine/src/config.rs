//! Default factory configuration — routing table defaults.
//!
//! The kernel ships a curated default routing table. Projects can override
//! individual entries by editing `.claude-factory/routing.toml` (M2+).

use cfk_core::types::routing::{
    AgentName, ClaudeModel, CodexEffort, CodexModel, ExecutorSpec, RoutingEntry, RoutingTable,
    WorkType,
};

/// Return the default kernel routing table.
///
/// Cross-family review (Codex/GPT for test and implementation review) is a
/// deliberate design: the reviewer must not share the author's blind spots.
#[must_use]
pub fn default_routing_table() -> RoutingTable {
    #[expect(clippy::expect_used, reason = "caller passes hardcoded string literals that are always valid")]
    fn claude(model: ClaudeModel, agent: &str) -> ExecutorSpec {
        ExecutorSpec::Claude {
            model,
            agent_name: AgentName::try_new(agent.to_owned()).expect("hardcoded agent name"),
        }
    }

    #[expect(clippy::expect_used, reason = "caller passes hardcoded string literals that are always valid")]
    fn codex(model: &str, effort: CodexEffort) -> ExecutorSpec {
        ExecutorSpec::Codex {
            model: CodexModel::try_new(model.to_owned()).expect("hardcoded model name"),
            effort,
        }
    }

    RoutingTable {
        entries: vec![
            RoutingEntry {
                work_type: WorkType::SocraticDiscovery,
                executor: claude(ClaudeModel::Opus, "discovery-partner"),
                notes: Some("Conversation quality matters; use the highest-capability model.".into()),
            },
            RoutingEntry {
                work_type: WorkType::EventModelAuthoring,
                executor: claude(ClaudeModel::Sonnet, "event-modeler"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::ArchitectureTriage,
                executor: claude(ClaudeModel::Sonnet, "architecture-triage"),
                notes: Some("Per-slice architecture gate: decide whether an ADR is required.".into()),
            },
            RoutingEntry {
                work_type: WorkType::AdrDrafting,
                executor: claude(ClaudeModel::Sonnet, "architect"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::AdrReview,
                executor: codex("gpt-5.5", CodexEffort::High),
                notes: Some("Cross-family ADR review: GPT checks for conflicts from a different perspective.".into()),
            },
            RoutingEntry {
                work_type: WorkType::DesignTriage,
                executor: claude(ClaudeModel::Sonnet, "design-triage"),
                notes: Some("Per-slice design gate: decide whether UI components must be built.".into()),
            },
            RoutingEntry {
                work_type: WorkType::DesignSystemBuild,
                executor: claude(ClaudeModel::Sonnet, "design-system-builder"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::OuterBehavioralTestWriting,
                executor: claude(ClaudeModel::Sonnet, "test-writer"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::TestReview,
                executor: codex("gpt-5.5", CodexEffort::High),
                notes: Some("Cross-family review: GPT catches different blind spots than Claude.".into()),
            },
            RoutingEntry {
                work_type: WorkType::NarrowestStepImplementation,
                executor: claude(ClaudeModel::Sonnet, "implementer"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::ImplementationReview,
                executor: codex("gpt-5.5", CodexEffort::High),
                notes: Some("Cross-family review.".into()),
            },
            RoutingEntry {
                work_type: WorkType::MechanicalTransform,
                executor: claude(ClaudeModel::Haiku, "implementer"),
                notes: Some("Mechanical work (renames, scaffolds) needs only a fast model.".into()),
            },
            RoutingEntry {
                work_type: WorkType::PrCommentTriage,
                executor: claude(ClaudeModel::Sonnet, "pr-shepherd-triage"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::Research,
                executor: claude(ClaudeModel::Haiku, "researcher"),
                notes: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::default_routing_table;
    use cfk_core::types::routing::{CodexModel, ExecutorSpec, WorkType};

    /// Every work type the kernel can dispatch must have a routing entry, or
    /// `cf_next_step` fails to build a step. The per-slice triage gates
    /// (ADR 0011) are dispatchable work types and must resolve.
    #[test]
    fn default_routing_resolves_every_work_type() {
        let table = default_routing_table();
        for work_type in [
            WorkType::SocraticDiscovery,
            WorkType::EventModelAuthoring,
            WorkType::ArchitectureTriage,
            WorkType::AdrDrafting,
            WorkType::AdrReview,
            WorkType::DesignTriage,
            WorkType::DesignSystemBuild,
            WorkType::OuterBehavioralTestWriting,
            WorkType::TestReview,
            WorkType::NarrowestStepImplementation,
            WorkType::ImplementationReview,
            WorkType::MechanicalTransform,
            WorkType::PrCommentTriage,
            WorkType::Research,
        ] {
            assert!(
                table.resolve(work_type).is_some(),
                "no routing entry for {work_type:?}"
            );
        }
    }

    /// The Codex CLI rejects the model `o4-mini` under a ChatGPT account, so the
    /// default routing table must never specify it for any work type. Review work
    /// types use `gpt-5.5` instead.
    #[test]
    #[expect(clippy::expect_used, reason = "test constructs a known-valid identifier")]
    fn default_routing_never_uses_o4_mini() {
        let table = default_routing_table();
        let o4_mini =
            CodexModel::try_new("o4-mini".to_string()).expect("valid CodexModel identifier");

        for entry in &table.entries {
            if let ExecutorSpec::Codex { model, .. } = &entry.executor {
                assert_ne!(
                    model, &o4_mini,
                    "work type {:?} routes to the unsupported Codex model o4-mini",
                    entry.work_type
                );
            }
        }
    }
}
