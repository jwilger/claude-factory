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
                executor: codex("o4-mini", CodexEffort::High),
                notes: Some("Cross-family review: GPT catches different blind spots than Claude.".into()),
            },
            RoutingEntry {
                work_type: WorkType::NarrowestStepImplementation,
                executor: claude(ClaudeModel::Sonnet, "implementer"),
                notes: None,
            },
            RoutingEntry {
                work_type: WorkType::ImplementationReview,
                executor: codex("o4-mini", CodexEffort::High),
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
