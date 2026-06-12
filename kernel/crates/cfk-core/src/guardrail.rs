//! Pure decision logic for the product-source edit guardrail.
//!
//! The guardrail answers one question: may this session edit this file right
//! now? It is enforced by a `PreToolUse` command hook that shells out to
//! `cfk guardrail-check`, but the *policy* lives here as a pure function over
//! pre-computed facts, so it is deterministic and exhaustively testable without
//! any I/O. The imperative shell (cfk-engine / cfk-mcp) gathers the facts —
//! whether the project is factory-managed, whether the operator bypass sentinel
//! is present, whether the path matches the project's protected globs, and
//! whether the editing session holds an active lease — and calls [`decide`].

use serde::{Deserialize, Serialize};

/// Why the guardrail allowed an edit. Carried for transparency in messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowReason {
    /// The directory is not a Claude-Factory project — nothing to guard.
    NotFactoryProject,
    /// An operator bypass sentinel (`LEASE_BYPASS`) is present for the project.
    BypassActive,
    /// The path does not match any of the project's protected globs.
    PathNotProtected,
    /// The editing session holds an active lease.
    LeaseHeld,
}

/// Why the guardrail blocked an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    /// The path is protected product source and the session holds no active
    /// lease — the edit must go through the factory's TDD workflow.
    NoActiveLeaseForSession,
}

/// The guardrail's verdict for a single edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardrailDecision {
    Allow(AllowReason),
    Block(BlockReason),
}

impl GuardrailDecision {
    /// True when the edit is permitted.
    #[must_use]
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allow(_))
    }
}

/// The pre-computed facts the policy decides over. Each field is gathered by the
/// imperative shell; this struct carries no I/O.
#[expect(
    clippy::struct_excessive_bools,
    reason = "these are four independent, gathered boolean facts that the pure policy decides over; collapsing them into an enum would lose the orthogonal precedence the decision relies on"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardrailFacts {
    /// The edit's directory tree is a Claude-Factory project (`.claude-factory/`
    /// exists at the resolved project root).
    pub is_factory_project: bool,
    /// The operator bypass sentinel is present for the project.
    pub bypass_active: bool,
    /// The edited path matches at least one of the project's protected globs.
    pub path_is_protected: bool,
    /// The editing session holds an active (non-expired) lease.
    pub session_holds_active_lease: bool,
}

/// Decide whether an edit is permitted, given the gathered facts.
///
/// The precedence is deliberate and short-circuiting:
/// 1. Not a factory project → allow (the guardrail only governs factory repos).
/// 2. Operator bypass present → allow (explicit, temporary override).
/// 3. Path not protected → allow (only product source is gated).
/// 4. Session holds an active lease → allow; otherwise block.
#[must_use]
pub fn decide(facts: GuardrailFacts) -> GuardrailDecision {
    if !facts.is_factory_project {
        return GuardrailDecision::Allow(AllowReason::NotFactoryProject);
    }
    if facts.bypass_active {
        return GuardrailDecision::Allow(AllowReason::BypassActive);
    }
    if !facts.path_is_protected {
        return GuardrailDecision::Allow(AllowReason::PathNotProtected);
    }
    if facts.session_holds_active_lease {
        return GuardrailDecision::Allow(AllowReason::LeaseHeld);
    }
    GuardrailDecision::Block(BlockReason::NoActiveLeaseForSession)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All facts "on" except as overridden — the base case is a protected edit
    /// in a factory project with no bypass and no lease, which must block.
    fn protected_unleased() -> GuardrailFacts {
        GuardrailFacts {
            is_factory_project: true,
            bypass_active: false,
            path_is_protected: true,
            session_holds_active_lease: false,
        }
    }

    #[test]
    fn blocks_protected_edit_without_lease() {
        assert_eq!(
            decide(protected_unleased()),
            GuardrailDecision::Block(BlockReason::NoActiveLeaseForSession)
        );
    }

    #[test]
    fn allows_when_session_holds_lease() {
        let facts = GuardrailFacts {
            session_holds_active_lease: true,
            ..protected_unleased()
        };
        assert_eq!(decide(facts), GuardrailDecision::Allow(AllowReason::LeaseHeld));
    }

    #[test]
    fn allows_when_path_not_protected() {
        let facts = GuardrailFacts {
            path_is_protected: false,
            ..protected_unleased()
        };
        assert_eq!(decide(facts), GuardrailDecision::Allow(AllowReason::PathNotProtected));
    }

    #[test]
    fn bypass_overrides_missing_lease() {
        let facts = GuardrailFacts {
            bypass_active: true,
            ..protected_unleased()
        };
        assert_eq!(decide(facts), GuardrailDecision::Allow(AllowReason::BypassActive));
    }

    #[test]
    fn non_factory_project_is_never_guarded() {
        let facts = GuardrailFacts {
            is_factory_project: false,
            ..protected_unleased()
        };
        assert_eq!(decide(facts), GuardrailDecision::Allow(AllowReason::NotFactoryProject));
    }

    #[test]
    fn bypass_takes_precedence_over_protection_and_lease() {
        // Bypass short-circuits before the protection/lease checks.
        let facts = GuardrailFacts {
            is_factory_project: true,
            bypass_active: true,
            path_is_protected: true,
            session_holds_active_lease: false,
        };
        assert!(decide(facts).is_allowed());
    }
}
