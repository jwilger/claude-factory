//! Pure prompt-building functions for each workflow phase.
//!
//! Every function takes only string/domain data and returns a `StepPrompt` or
//! `HumanQuestion`. The `make_prompt`/`make_question` helpers carry a documented
//! `#[expect(clippy::expect_used)]` — every format string here has a non-empty
//! literal prefix, so the inner nutype validation can never fail.

use crate::types::step::{HumanQuestion, StepPrompt};

fn make_prompt(text: String) -> StepPrompt {
    #[expect(
        clippy::expect_used,
        reason = "all callers pass format strings with non-empty literal prefixes; empty output is structurally impossible"
    )]
    StepPrompt::try_new(text).expect("prompt text is non-empty")
}

fn make_question(text: String) -> HumanQuestion {
    #[expect(
        clippy::expect_used,
        reason = "all callers pass format strings with non-empty literal prefixes; empty output is structurally impossible"
    )]
    HumanQuestion::try_new(text).expect("question text is non-empty")
}

/// Generic executor step: the item description IS the prompt.
#[must_use]
pub fn generic_step(description: &str) -> StepPrompt {
    make_prompt(description.to_string())
}

#[must_use]
pub fn tdd_write_test(description: &str) -> StepPrompt {
    make_prompt(format!(
        "Write an outer behavioural test for: {description}\n\nRequirements:\n\
         - Test must be behavioural (tests what the system does, not how)\n\
         - No mocks; use real I/O substitutes\n\
         - Use semantic types in test code\n\
         - The FIRST test for a slice must cover the PRIMARY SUCCESS scenario (the\n\
         happy path) and assert every value the success return type promises, so no\n\
         field ships unobservable. List the Given/When/Then scenarios you are NOT\n\
         covering (error/edge cases) so they can be scheduled next.\n\
         - Submit the complete test code in `test_content`."
    ))
}

#[must_use]
pub fn tdd_test_review(description: &str, test_content: &str) -> StepPrompt {
    make_prompt(format!(
        "Review this test for the slice: {description}\n\nTest code:\n```\n{test_content}\n```\n\n\
         Checklist:\n\
         - Is it behavioural (not implementation-coupled)?\n\
         - Does it use no mocking libraries?\n\
         - Does it use semantic types?\n\
         - Will it fail for the right reason?\n\
         Return verdict: approved or vetoed with reason."
    ))
}

#[must_use]
pub fn tdd_implement(first_error: &str) -> StepPrompt {
    make_prompt(format!(
        "Implement the narrowest change to address ONLY this error:\n\n\
         ```\n{first_error}\n```\n\n\
         Do NOT fix other errors. Do NOT refactor beyond what is required.\n\
         If this error requires changes to more than one function boundary,\n\
         set `drill_down_description` to describe the tighter unit test needed.\n\
         Otherwise leave `drill_down_description` null."
    ))
}

#[must_use]
pub fn tdd_impl_review(description: &str) -> StepPrompt {
    make_prompt(format!(
        "Review the implementation for the slice: {description}\n\n\
         Checklist (veto on any violation):\n\
         - Narrowest possible change; no unrelated refactoring; no mocking introduced.\n\
         - Functional core / imperative shell: business logic pure, I/O only at the boundary.\n\
         - Semantic types throughout — including struct fields and error payloads. Their\n\
         constructors must enforce invariants and return Result (parse, don't validate);\n\
         veto unchecked casts or pass-through constructors that admit illegal values.\n\
         - Railway-oriented errors: fallible ops return Result; no unwrap/expect/panic in\n\
         product code.\n\
         - Observable contract: every field/variant/return value of a public type is\n\
         reachable by callers and asserted by a test (no private accessorless success field).\n\
         Return verdict: approved or vetoed with reason."
    ))
}

#[must_use]
pub fn review_open_pr(description: &str) -> StepPrompt {
    make_prompt(format!(
        "Open a pull request for the slice: {description}\n\n\
         Provide a descriptive title and body. Submit via `cf_pr_open`."
    ))
}

#[must_use]
pub fn review_triage_comment(
    item_description: &str,
    comment_id: &str,
    comment_body: &str,
) -> StepPrompt {
    make_prompt(format!(
        "Respond to this PR review comment for the slice: {item_description}\n\n\
         Comment ID: {comment_id}\n\
         Comment: {comment_body}\n\n\
         Write a concise, professional reply. Submit via `cf_submit`."
    ))
}

#[must_use]
pub fn discovery_socratic(description: &str) -> StepPrompt {
    make_prompt(format!(
        "Run a socratic discovery dialogue for: {description}\n\n\
         Explore value, usability, feasibility, and viability risks.\n\
         Enumerate the key workflows and user journeys.\n\
         When done, submit via `cf_discovery_submit` with:\n\
         - `brief_content`: a concise product brief covering risks and opportunities\n\
         - `workflows`: list of workflow names for event modeling"
    ))
}

#[must_use]
pub fn discovery_brief_approval(description: &str, brief: &str) -> HumanQuestion {
    make_question(format!(
        "Discovery brief ready for: {description}\n\n\
         Brief:\n{brief}\n\n\
         Approve to queue workflows for event modeling, or reject to re-run discovery."
    ))
}

#[must_use]
pub fn architecture_triage(description: &str, accepted_summary: &str) -> StepPrompt {
    make_prompt(format!(
        "Architecture triage for slice: {description}\n\n\
         Accepted ADRs (authoritative — this list, not prior memory, defines what \
         is already decided):\n{accepted_summary}\n\n\
         Decide whether building this slice forces a NEW or CHANGED cross-cutting \
         architectural decision not already settled above. Warranting categories:\n\
         - persistence / storage strategy and schema evolution\n\
         - module / service boundaries and layering\n\
         - external protocols and integration contracts; cross-slice event schemas\n\
         - concurrency / consistency model, transaction boundaries\n\
         - execution / runtime model (schedulers, background workers, queues)\n\
         - a deliberate, scoped relaxation of the engineering baseline\n\
         If NO persistence/storage decision is accepted yet, the first slice that \
         needs durable state must raise one. Most later slices simply apply existing \
         decisions and fast-pass, even when they are substantial work.\n\
         This is an interactive decision: when the call is non-obvious, confirm with \
         the operator before deciding.\n\
         - No new decision needed → fast-pass.\n\
         - A decision is needed → an ADR will be drafted and reviewed next.\n\
         Submit via `cf_triage_submit` with `needs_followup` (true iff an ADR is \
         required) and a one-paragraph `rationale`."
    ))
}

#[must_use]
pub fn design_triage(description: &str, inventory_summary: &str) -> StepPrompt {
    make_prompt(format!(
        "Design triage for slice: {description}\n\n\
         Existing design inventory (each entry is `name (AtomicKind)`):\n\
         {inventory_summary}\n\n\
         Decide whether this slice needs UI components built.\n\
         1. Does the slice have a UI surface? Judge by what the slice DESCRIBES — a \
         screen, form, dashboard, or view a person uses — NOT by its kind label: a \
         `command` slice can still own a data-entry screen, and a `state_view` may be \
         an internal read model with no screen. Slices with no screen surface \
         fast-pass.\n\
         2. If it has a UI surface, enumerate the quarks → atoms → molecules → \
         organisms → templates → pages it needs and compare against the inventory \
         above. If every element already exists, fast-pass.\n\
         This is an interactive decision: collaborate with the operator on UX gaps.\n\
         - No UI surface, or inventory already sufficient → fast-pass.\n\
         - Components are missing → they will be built next (reusable elements to the \
         platform UI library; slice-specific ones owned by the slice).\n\
         Submit via `cf_triage_submit` with `needs_followup` (true iff components must \
         be built) and a one-paragraph `rationale`."
    ))
}

#[must_use]
pub fn architecture_draft_adr(description: &str, accepted_summary: &str) -> StepPrompt {
    make_prompt(format!(
        "Draft an Architecture Decision Record for: {description}\n\n\
         Existing accepted ADRs:\n{accepted_summary}\n\n\
         Follow ADR format: Context, Decision, Consequences.\n\
         Submit via `cf_adr_submit` with `title` and `content`."
    ))
}

#[must_use]
pub fn architecture_review_adr(title: &str, content: &str) -> StepPrompt {
    make_prompt(format!(
        "Review this ADR for conflicts with the factory engineering baseline and accepted ADRs:\n\n\
         **{title}**\n\n{content}\n\n\
         Check for contradictions with:\n\
         - Event modeling / event sourcing requirements\n\
         - Functional-core / imperative-shell architecture\n\
         - Railway-oriented programming for errors\n\
         - Semantic types (no raw primitives)\n\
         - Strictest-possible linting\n\
         - Behavioral tests only (no mocking)\n\
         - Atomic Design for UI\n\
         - Vertical slice architecture\n\
         Return verdict: approved or vetoed with reason."
    ))
}

#[must_use]
pub fn design_build_component(description: &str, inventory_summary: &str) -> StepPrompt {
    make_prompt(format!(
        "Build a design component for: {description}\n\n\
         Existing inventory:\n{inventory_summary}\n\n\
         Specify the Atomic Design level (quark/atom/molecule/organism/template/page),\n\
         the component name, the OWNERSHIP, and any relevant implementation notes.\n\
         Ownership (ADR 0012): `platform` if the component is reusable across slices\n\
         (typically lower levels — quarks, atoms, molecules, common organisms), or\n\
         `slice` if it is bespoke to this slice (typically its pages and unique\n\
         organisms). Submit via `cf_design_add_component` with `ownership`."
    ))
}
