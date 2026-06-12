//! Behavioral tests for cfk-engine.
//!
//! Tests follow the pattern: given events → when command → then result.
//! No I/O mocks — the event store uses a real temp directory.
//! Restart-durability tests kill the in-memory state and replay from disk.

#[cfg(test)]
mod work_item_claim {
    use crate::{
        commands::{handle_claim, handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event, load_events},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::work_item::{WorkItem, WorkItemStatus},
        types::{
            ids::{ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
        },
    };

    fn test_project(root: &std::path::Path) -> ProjectState {
        ProjectState::new(ProjectId::new(), root.to_path_buf(), default_routing_table())
    }

    fn seed_work_item(root: &std::path::Path, project: &mut ProjectState, seq: &mut u64) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::OuterBehavioralTestWriting,
            "Write outer behavioral test for the login slice.".to_string(),
        );
        let id = item.id.clone();
        *seq += 1;
        let envelope = append_event(root, *seq, FactoryEvent::WorkItemAdded { work_item: item })
            .expect("append event");
        apply_event(project, &envelope.payload);
        id
    }

    #[test]
    fn next_step_returns_ready_for_seeded_item() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        let mut project = test_project(root);
        let mut seq = 0u64;
        seed_work_item(root, &mut project, &mut seq);

        let response = handle_next_step(&project, None).expect("handle_next_step");
        assert!(
            matches!(response, NextStepResponse::Ready(_)),
            "expected Ready, got {response:?}"
        );
    }

    #[test]
    fn next_step_returns_idle_when_no_work() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project = test_project(dir.path());

        let response = handle_next_step(&project, None).expect("handle_next_step");
        assert!(
            matches!(response, NextStepResponse::Idle(_)),
            "expected Idle, got {response:?}"
        );
    }

    #[test]
    fn claim_transitions_item_to_in_progress() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        let mut project = test_project(root);
        let mut seq = 0u64;
        let work_item_id = seed_work_item(root, &mut project, &mut seq);

        let lease = handle_claim(&project, &work_item_id, "test-session").expect("handle_claim");

        seq += 1;
        let envelope = append_event(
            root,
            seq,
            FactoryEvent::LeaseGranted {
                lease: lease.clone(),
            },
        )
        .expect("append event");
        apply_event(&mut project, &envelope.payload);

        let item = project.work_items.iter().find(|i| i.id == work_item_id).unwrap();
        assert_eq!(item.status, WorkItemStatus::InProgress);
        assert_eq!(item.active_lease, Some(lease.id));
    }

    #[test]
    fn next_step_skips_in_progress_items() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        let mut project = test_project(root);
        let mut seq = 0u64;
        let work_item_id = seed_work_item(root, &mut project, &mut seq);

        // Claim the only item.
        let lease = handle_claim(&project, &work_item_id, "test-session").expect("handle_claim");
        seq += 1;
        let envelope = append_event(root, seq, FactoryEvent::LeaseGranted { lease })
            .expect("append event");
        apply_event(&mut project, &envelope.payload);

        // Now next_step should return Idle.
        let response = handle_next_step(&project, None).expect("handle_next_step");
        assert!(
            matches!(response, NextStepResponse::Idle(_)),
            "expected Idle after claiming the only item"
        );
    }

    #[test]
    fn release_returns_item_to_ready() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        let mut project = test_project(root);
        let mut seq = 0u64;
        let work_item_id = seed_work_item(root, &mut project, &mut seq);

        let lease = handle_claim(&project, &work_item_id, "test-session").expect("handle_claim");
        let lease_id = lease.id.clone();
        seq += 1;
        let envelope = append_event(root, seq, FactoryEvent::LeaseGranted { lease })
            .expect("append event");
        apply_event(&mut project, &envelope.payload);

        // Release the lease.
        seq += 1;
        let envelope = append_event(
            root,
            seq,
            FactoryEvent::LeaseReleased {
                lease_id,
                work_item_id: work_item_id.clone(),
            },
        )
        .expect("append event");
        apply_event(&mut project, &envelope.payload);

        let item = project.work_items.iter().find(|i| i.id == work_item_id).unwrap();
        assert_eq!(item.status, WorkItemStatus::Ready);
        assert_eq!(item.active_lease, None);
    }

    #[test]
    fn events_persist_and_replay_correctly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        // Initialize project by writing the first event.
        let project_id = ProjectId::new();
        append_event(root, 1, FactoryEvent::ProjectInitialized { id: project_id })
            .expect("write init event");

        // Add a work item.
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::NarrowestStepImplementation,
            "Implement the login command handler.".to_string(),
        );
        append_event(root, 2, FactoryEvent::WorkItemAdded { work_item: item })
            .expect("write work item event");

        // Load from disk and verify.
        let project = load_project_state(root).expect("load_project_state").expect("Some");
        assert_eq!(project.work_items.len(), 1);
        assert_eq!(project.work_items[0].status, WorkItemStatus::Ready);

        // Verify the event files exist and are loadable.
        let events = load_events(&crate::store::event_export_dir(root)).expect("load_events");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn phase_filter_restricts_next_step() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        let mut project = test_project(root);
        let mut seq = 0u64;

        // Add a Development item.
        let dev_item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::OuterBehavioralTestWriting,
            "Dev task.".to_string(),
        );
        seq += 1;
        let envelope = append_event(root, seq, FactoryEvent::WorkItemAdded { work_item: dev_item })
            .expect("append event");
        apply_event(&mut project, &envelope.payload);

        // Requesting a Discovery-phase step should return Idle.
        let response = handle_next_step(&project, Some(PhaseKind::Discovery)).expect("handle");
        assert!(matches!(response, NextStepResponse::Idle(_)));

        // Requesting a Development-phase step should return Ready.
        let response = handle_next_step(&project, Some(PhaseKind::Development)).expect("handle");
        assert!(matches!(response, NextStepResponse::Ready(_)));
    }
}

// ── TDD state machine behavioral tests ───────────────────────────────────────

#[cfg(test)]
mod tdd_slice {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::work_item::WorkItem,
        types::{
            gate::{GateKind, GateVerdict, VetoReason},
            ids::{ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
            tdd::TddPhase,
        },
    };

    fn test_project(root: &std::path::Path) -> ProjectState {
        ProjectState::new(ProjectId::new(), root.to_path_buf(), default_routing_table())
    }

    fn seed_dev_slice(root: &std::path::Path, project: &mut ProjectState, seq: &mut u64) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::OuterBehavioralTestWriting,
            "Add `add(a, b)` function to toy-product lib".to_string(),
        );
        let id = item.id.clone();
        *seq += 1;
        let env = append_event(root, *seq, FactoryEvent::WorkItemAdded { work_item: item })
            .expect("append WorkItemAdded");
        apply_event(project, &env.payload);
        id
    }

    fn claim_and_start_tdd(
        root: &std::path::Path,
        project: &mut ProjectState,
        seq: &mut u64,
        wid: &WorkItemId,
        session: &str,
    ) {
        use crate::commands::handle_claim;
        let lease = handle_claim(project, wid, session).expect("claim");
        *seq += 1;
        let env = append_event(root, *seq, FactoryEvent::LeaseGranted { lease })
            .expect("append LeaseGranted");
        apply_event(project, &env.payload);

        *seq += 1;
        let env = append_event(root, *seq, FactoryEvent::TddSliceStarted {
            work_item_id: wid.clone(),
            author_identity: session.to_string(),
        }).expect("append TddSliceStarted");
        apply_event(project, &env.payload);
    }

    // ── test: TDD slice starts in WriteTest ───────────────────────────────

    #[test]
    fn after_claim_dev_slice_is_in_write_test() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut project = test_project(root);
        let mut seq = 0u64;

        let wid = seed_dev_slice(root, &mut project, &mut seq);
        claim_and_start_tdd(root, &mut project, &mut seq, &wid, "author-session");

        assert_eq!(
            project.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::WriteTest)
        );
    }

    // ── test: cf_next_step returns spawn_agent for WriteTest ──────────────

    #[test]
    fn next_step_for_in_progress_dev_slice_returns_spawn_agent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut project = test_project(root);
        let mut seq = 0u64;

        let wid = seed_dev_slice(root, &mut project, &mut seq);
        claim_and_start_tdd(root, &mut project, &mut seq, &wid, "author-session");

        let response = handle_next_step(&project, None).expect("next_step");
        let step = match response {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready, got Idle"),
        };
        assert_eq!(step.work_item_id, wid);
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }));
    }

    // ── test: submit test → TestReviewGate ───────────────────────────────

    #[test]
    fn submit_test_advances_to_test_review_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut project = test_project(root);
        let mut seq = 0u64;

        let wid = seed_dev_slice(root, &mut project, &mut seq);
        claim_and_start_tdd(root, &mut project, &mut seq, &wid, "author-session");

        // Emit TddTestSubmitted
        seq += 1;
        let env = append_event(root, seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 0,
            test_content: "#[test] fn test_add() { assert_eq!(add(1, 2), 3); }".to_string(),
            author_identity: "author-session".to_string(),
        }).expect("append TddTestSubmitted");
        apply_event(&mut project, &env.payload);

        assert_eq!(
            project.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::TestReviewGate)
        );
    }

    // ── test: gate approve advances to RedCheck ──────────────────────────

    #[test]
    fn gate_approve_advances_to_red_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut project = test_project(root);
        let mut seq = 0u64;

        let wid = seed_dev_slice(root, &mut project, &mut seq);
        claim_and_start_tdd(root, &mut project, &mut seq, &wid, "author-session");

        seq += 1;
        apply_event(&mut project, &append_event(root, seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 0,
            test_content: "test".to_string(),
            author_identity: "author-session".to_string(),
        }).expect("TddTestSubmitted").payload);

        seq += 1;
        apply_event(&mut project, &append_event(root, seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::TestReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "reviewer-session".to_string(),
        }).expect("TddGateVerdict").payload);

        assert_eq!(
            project.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::RedCheck)
        );
    }

    // ── test: veto loops back to WriteTest ───────────────────────────────

    #[test]
    fn gate_veto_loops_back_to_write_test() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut project = test_project(root);
        let mut seq = 0u64;

        let wid = seed_dev_slice(root, &mut project, &mut seq);
        claim_and_start_tdd(root, &mut project, &mut seq, &wid, "author-session");

        seq += 1;
        apply_event(&mut project, &append_event(root, seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 0,
            test_content: "test".to_string(),
            author_identity: "author-session".to_string(),
        }).expect("TddTestSubmitted").payload);

        let reason = VetoReason::try_new("Test is implementation-coupled".to_string())
            .expect("valid reason");
        seq += 1;
        apply_event(&mut project, &append_event(root, seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::TestReview,
            verdict: GateVerdict::Vetoed { reason },
            reviewer_id: "reviewer-session".to_string(),
        }).expect("TddGateVerdict").payload);

        assert_eq!(
            project.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::WriteTest),
            "veto should loop back to WriteTest"
        );
    }

    // ── test: check result in RedCheck advances to Implement ─────────────

    #[test]
    fn red_check_failing_advances_to_implement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut project = test_project(root);
        let mut seq = 0u64;

        let wid = seed_dev_slice(root, &mut project, &mut seq);
        claim_and_start_tdd(root, &mut project, &mut seq, &wid, "author-session");
        // Manually advance to RedCheck for this test.
        project.dev_states.get_mut(&wid).unwrap().current_frame_mut().unwrap().phase = TddPhase::RedCheck;

        seq += 1;
        apply_event(&mut project, &append_event(root, seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "tests".to_string(),
            passed: false,
            first_error: Some("error[E0425]: cannot find function `add`".to_string()),
        }).expect("TddCheckResult").payload);

        assert_eq!(
            project.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::Implement)
        );
        assert!(
            project.dev_states.get(&wid)
                .and_then(|d| d.current_frame())
                .and_then(|f| f.current_error.as_deref())
                .is_some()
        );
    }

    // ── test: restart durability (replay from events) ────────────────────

    #[test]
    fn tdd_state_survives_restart_via_event_replay() {
        use cfk_core::types::{ids::LeaseId, lease::{Lease, SessionIdentity}};
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        // Session 1: initialize project, seed work item, claim, start TDD.
        let project_id = ProjectId::new();
        let mut seq = 0u64;

        seq += 1;
        append_event(root, seq, FactoryEvent::ProjectInitialized { id: project_id })
            .expect("ProjectInitialized");

        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::OuterBehavioralTestWriting,
            "Implement add function".to_string(),
        );
        let wid = item.id.clone();
        seq += 1;
        append_event(root, seq, FactoryEvent::WorkItemAdded { work_item: item })
            .expect("WorkItemAdded");

        // Fake a lease (no actual claim handling — just append the events).
        let lease = Lease {
            id: LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: SessionIdentity::try_new("s1".to_string()).expect("id"),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        seq += 1;
        append_event(root, seq, FactoryEvent::LeaseGranted { lease }).expect("LeaseGranted");

        seq += 1;
        append_event(root, seq, FactoryEvent::TddSliceStarted {
            work_item_id: wid.clone(),
            author_identity: "s1".to_string(),
        }).expect("TddSliceStarted");

        // Advance to TestReviewGate.
        seq += 1;
        append_event(root, seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 0,
            test_content: "the test".to_string(),
            author_identity: "s1".to_string(),
        }).expect("TddTestSubmitted");

        // --- Simulate restart: rebuild state from events ---
        let state = load_project_state(root).expect("load").expect("Some");

        // TDD phase must be preserved across restart.
        assert_eq!(
            state.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::TestReviewGate),
            "TDD phase must survive event replay"
        );

        // next_step should still produce a gate_review action.
        let response = handle_next_step(&state, None).expect("next_step");
        match response {
            NextStepResponse::Ready(step) => {
                assert!(
                    matches!(step.action, StepAction::GateReview { .. }),
                    "expected GateReview step after restart, got {:?}", step.action
                );
            }
            NextStepResponse::Idle(_) => panic!("expected Ready after restart"),
        }
    }
}

// ── M2 exit-criterion test ────────────────────────────────────────────────────
// Full slice lifecycle: claim → WriteTest → veto loop → approve → RedCheck →
// Implement → drill-down (child TDD cycle) → pop → Implement → green →
// ImplReviewGate → LintCheck → Done.
// Includes one simulated restart mid-journey to verify event-replay durability.

#[cfg(test)]
mod m2_full_slice {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::work_item::{WorkItem, WorkItemStatus},
        types::{
            gate::{GateKind, GateVerdict, VetoReason},
            ids::{LeaseId, ProjectId, WorkItemId},
            lease::{Lease, SessionIdentity},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
            tdd::TddPhase,
        },
    };

    fn append<P: AsRef<std::path::Path>>(
        root: P,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root.as_ref(), *seq, event).expect("append_event");
        apply_event(state, &env.payload);
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn full_slice_lifecycle_with_veto_and_drilldown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let project_id = ProjectId::new();
        let mut seq = 0u64;

        // ── Initialise ────────────────────────────────────────────────────────
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        append(root, &mut seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);

        // ── Seed work item ───────────────────────────────────────────────────
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::OuterBehavioralTestWriting,
            "Implement multiply(a, b) for toy-product".to_string(),
        );
        let wid = item.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: item }, &mut state);

        // ── Claim (author = "alice") ──────────────────────────────────────────
        let lease = Lease {
            id: LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: SessionIdentity::try_new("alice".to_string()).expect("alice"),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);
        append(root, &mut seq, FactoryEvent::TddSliceStarted {
            work_item_id: wid.clone(),
            author_identity: "alice".to_string(),
        }, &mut state);

        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::WriteTest));

        // cf_next_step → SpawnAgent (WriteTest)
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for WriteTest"),
        };
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }), "WriteTest must be SpawnAgent");

        // ── Submit test (first attempt) ───────────────────────────────────────
        append(root, &mut seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 0,
            test_content: "#[test] fn test_multiply() { assert_eq!(multiply(3, 4), 12); }".to_string(),
            author_identity: "alice".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::TestReviewGate));

        // cf_next_step → GateReview
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for TestReviewGate"),
        };
        assert!(matches!(step.action, StepAction::GateReview { .. }), "TestReviewGate must be GateReview");

        // ── Veto loop: reviewer "bob" vetoes the first test ───────────────────
        let reason = VetoReason::try_new("Test is implementation-coupled".to_string()).expect("reason");
        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::TestReview,
            verdict: GateVerdict::Vetoed { reason },
            reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(
            state.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::WriteTest),
            "veto must loop back to WriteTest"
        );

        // ── Submit revised test ───────────────────────────────────────────────
        append(root, &mut seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 0,
            test_content: "#[test] fn multiplying_returns_product() { assert_eq!(multiply(3, 4), 12); }".to_string(),
            author_identity: "alice".to_string(),
        }, &mut state);

        // ── Gate approved (bob reviews again) ────────────────────────────────
        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::TestReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::RedCheck));

        // ── RedCheck: test fails for the expected reason ─────────────────────
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "tests".to_string(),
            passed: false,
            first_error: Some("error[E0425]: cannot find function `multiply` in this scope".to_string()),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::Implement));

        // cf_next_step → SpawnAgent (Implement)
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for Implement"),
        };
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }), "Implement must be SpawnAgent");

        // ── RESTART: simulate mid-slice session kill ──────────────────────────
        drop(state); // discard in-memory state
        let state = load_project_state(root).expect("load").expect("Some");
        assert_eq!(
            state.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::Implement),
            "phase must survive restart"
        );

        // Reload mutable state from disk; continue appending events.
        let mut state = load_project_state(root).expect("reload").expect("Some");

        // ── Drill-down: implementer signals multi-function gap ────────────────
        // Parent stays at depth=0; child pushed at depth=1.
        append(root, &mut seq, FactoryEvent::TddDrillDownPushed {
            work_item_id: wid.clone(),
            child_depth: 1,
            child_description: "Need unit test for multiply_core helper".to_string(),
        }, &mut state);

        assert_eq!(
            state.dev_states.get(&wid).map(|d| d.frames.len()),
            Some(2),
            "drill-down must add a child frame"
        );
        assert_eq!(
            state.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::WriteTest),
            "child frame starts at WriteTest"
        );

        // ── Child TDD cycle ───────────────────────────────────────────────────
        // child: WriteTest
        append(root, &mut seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(),
            frame_depth: 1,
            test_content: "#[test] fn unit_multiply_core() { assert_eq!(multiply_core(3, 4), 12); }".to_string(),
            author_identity: "alice".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::TestReviewGate));

        // child: gate approved
        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::TestReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::RedCheck));

        // child: RedCheck fails
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "tests".to_string(),
            passed: false,
            first_error: Some("error[E0425]: cannot find function `multiply_core`".to_string()),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::Implement));

        // child: implement, advance to CheckProgress, then green
        append(root, &mut seq, FactoryEvent::TddPhaseAdvanced {
            work_item_id: wid.clone(),
            frame_depth: 1,
            new_phase: TddPhase::CheckProgress,
        }, &mut state);
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "tests".to_string(),
            passed: true,
            first_error: None,
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::ImplReviewGate));

        // child: impl review approved
        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::ImplementationReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::LintCheck));

        // child: lint passes → child Done
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "lint".to_string(),
            passed: true,
            first_error: None,
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::Done));

        // ── Pop drill-down: parent resumes at Implement ───────────────────────
        append(root, &mut seq, FactoryEvent::TddDrillDownPopped {
            work_item_id: wid.clone(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).map(|d| d.frames.len()), Some(1), "parent frame only");
        assert_eq!(
            state.dev_states.get(&wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::Implement),
            "parent resumes at Implement after drill-down"
        );

        // ── Parent: implement, green ──────────────────────────────────────────
        append(root, &mut seq, FactoryEvent::TddPhaseAdvanced {
            work_item_id: wid.clone(),
            frame_depth: 0,
            new_phase: TddPhase::CheckProgress,
        }, &mut state);
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "tests".to_string(),
            passed: true,
            first_error: None,
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::ImplReviewGate));

        // ── Parent: impl review gate ─────────────────────────────────────────
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for ImplReviewGate"),
        };
        assert!(matches!(step.action, StepAction::GateReview { gate_kind: GateKind::ImplementationReview, .. }));

        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(),
            gate_kind: GateKind::ImplementationReview,
            verdict: GateVerdict::Approved,
            reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::LintCheck));

        // ── Parent: lint passes ───────────────────────────────────────────────
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(),
            check_name: "lint".to_string(),
            passed: true,
            first_error: None,
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::Done));

        // ── Slice done → work item completed ─────────────────────────────────
        append(root, &mut seq, FactoryEvent::TddSliceDone { work_item_id: wid.clone() }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: wid.clone() }, &mut state);

        let item = state.work_items.iter().find(|i| i.id == wid).expect("item");
        assert_eq!(item.status, WorkItemStatus::Done, "work item must reach Done");

        // handle_next_step should now return Idle (no more ready work).
        let response = handle_next_step(&state, None).expect("next_step after done");
        assert!(
            matches!(response, NextStepResponse::Idle(_)),
            "no work remains after slice done"
        );
    }
}

#[cfg(test)]
mod emc_ingestion {
    use crate::emc::read_verified_slices;
    use std::fs;

    #[allow(clippy::needless_pass_by_value)]
    fn write_emc_event(dir: &std::path::Path, filename: &str, payload: serde_json::Value) {
        let content = serde_json::to_string_pretty(&payload).unwrap();
        fs::write(dir.join(filename), content).unwrap();
    }

    fn setup_emc_dir(root: &std::path::Path) -> std::path::PathBuf {
        let emc_dir = root.join("model").join("events").join("v1");
        fs::create_dir_all(&emc_dir).unwrap();
        emc_dir
    }

    #[test]
    fn empty_model_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let slices = read_verified_slices(tmp.path()).unwrap();
        assert!(slices.is_empty(), "no model dir → no slices");
    }

    #[test]
    fn slices_without_verified_workflow_are_excluded() {
        let tmp = tempfile::tempdir().unwrap();
        let emc_dir = setup_emc_dir(tmp.path());

        write_emc_event(&emc_dir, "0000000001-aaaa.json", serde_json::json!({
            "schema_version": "1",
            "event_id": "aaaa",
            "stream_id": "s1",
            "type": "SliceAdded",
            "payload": {
                "workflow": "user-login",
                "slug": "login-state-change",
                "name": "Login State Change",
                "kind": "state_change",
                "description": "Handles the login command and emits UserLoggedIn."
            }
        }));

        let slices = read_verified_slices(tmp.path()).unwrap();
        assert!(slices.is_empty(), "unverified workflow slices must not be ingested");
    }

    #[test]
    fn verified_workflow_slices_are_returned() {
        let tmp = tempfile::tempdir().unwrap();
        let emc_dir = setup_emc_dir(tmp.path());

        write_emc_event(&emc_dir, "0000000001-aaaa.json", serde_json::json!({
            "schema_version": "1",
            "event_id": "aaaa",
            "stream_id": "s1",
            "type": "SliceAdded",
            "payload": {
                "workflow": "user-login",
                "slug": "login-state-change",
                "name": "Login State Change",
                "kind": "state_change",
                "description": "Handles the login command and emits UserLoggedIn."
            }
        }));
        write_emc_event(&emc_dir, "0000000002-bbbb.json", serde_json::json!({
            "schema_version": "1",
            "event_id": "bbbb",
            "stream_id": "s1",
            "type": "SliceAdded",
            "payload": {
                "workflow": "user-login",
                "slug": "login-view",
                "name": "Login View",
                "kind": "state_view",
                "description": "Projects UserLoggedIn into the active-session read model."
            }
        }));
        write_emc_event(&emc_dir, "0000000003-cccc.json", serde_json::json!({
            "schema_version": "1",
            "event_id": "cccc",
            "stream_id": "s1",
            "type": "WorkflowReadinessDeclared",
            "payload": { "workflow": "user-login" }
        }));

        let slices = read_verified_slices(tmp.path()).unwrap();
        assert_eq!(slices.len(), 2);
        let slugs: Vec<&str> = slices.iter().map(|s| s.slug.as_str()).collect();
        assert!(slugs.contains(&"login-state-change"));
        assert!(slugs.contains(&"login-view"));
    }

    #[test]
    fn only_verified_workflows_included_when_multiple_workflows_present() {
        let tmp = tempfile::tempdir().unwrap();
        let emc_dir = setup_emc_dir(tmp.path());

        write_emc_event(&emc_dir, "0000000001-aaaa.json", serde_json::json!({
            "schema_version": "1", "event_id": "aaaa", "stream_id": "s1",
            "type": "SliceAdded",
            "payload": {
                "workflow": "verified-flow", "slug": "v-slice",
                "name": "V Slice", "kind": "state_change", "description": "desc"
            }
        }));
        write_emc_event(&emc_dir, "0000000002-bbbb.json", serde_json::json!({
            "schema_version": "1", "event_id": "bbbb", "stream_id": "s1",
            "type": "SliceAdded",
            "payload": {
                "workflow": "unverified-flow", "slug": "u-slice",
                "name": "U Slice", "kind": "state_change", "description": "desc"
            }
        }));
        write_emc_event(&emc_dir, "0000000003-cccc.json", serde_json::json!({
            "schema_version": "1", "event_id": "cccc", "stream_id": "s1",
            "type": "WorkflowReadinessDeclared",
            "payload": { "workflow": "verified-flow" }
        }));

        let slices = read_verified_slices(tmp.path()).unwrap();
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].slug, "v-slice");
    }

    #[test]
    fn dedup_by_emc_slug_prevents_double_ingestion() {
        use crate::{
            commands::handle_next_step,
            config::default_routing_table,
            events::{FactoryEvent, append_event},
            loader::apply_event,
            project::ProjectState,
        };
        use cfk_core::{
            state_machine::work_item::WorkItem,
            types::{ids::ProjectId, phase::PhaseKind, routing::WorkType},
        };

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let emc_dir = setup_emc_dir(root);

        write_emc_event(&emc_dir, "0000000001-aaaa.json", serde_json::json!({
            "schema_version": "1", "event_id": "aaaa", "stream_id": "s1",
            "type": "SliceAdded",
            "payload": {
                "workflow": "login", "slug": "login-slice",
                "name": "Login", "kind": "state_change", "description": "desc"
            }
        }));
        write_emc_event(&emc_dir, "0000000002-bbbb.json", serde_json::json!({
            "schema_version": "1", "event_id": "bbbb", "stream_id": "s1",
            "type": "WorkflowReadinessDeclared",
            "payload": { "workflow": "login" }
        }));

        let mut state = ProjectState::new(ProjectId::new(), root.to_path_buf(), default_routing_table());
        let mut seq = 0u64;

        // Simulate first ingestion: manually add the emc-sourced work item.
        let item = WorkItem::from_emc_slice(
            cfk_core::types::ids::WorkItemId::new(),
            PhaseKind::Development,
            WorkType::NarrowestStepImplementation,
            "[login-slice] Login".to_string(),
            "login-slice".to_string(),
        );
        seq += 1;
        let env = append_event(root, seq, FactoryEvent::WorkItemAdded { work_item: item }).unwrap();
        apply_event(&mut state, &env.payload);

        // Check existing slugs to simulate dedup logic.
        let existing: std::collections::HashSet<String> = state
            .work_items
            .iter()
            .filter_map(|i| i.emc_slug.clone())
            .collect();

        let slices = read_verified_slices(root).unwrap();
        let new_slices: Vec<_> = slices.iter().filter(|s| !existing.contains(&s.slug)).collect();

        assert!(new_slices.is_empty(), "already-ingested slug must be skipped on second ingest");

        // Verify exactly one work item in state.
        let response = handle_next_step(&state, None).unwrap();
        assert!(
            matches!(response, crate::commands::NextStepResponse::Ready(_)),
            "the single work item should be ready"
        );
    }
}

// ── Review phase behavioral tests ────────────────────────────────────────────

#[cfg(test)]
mod review_slice {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        forge::{MemoryForge, PollScript},
        loader::{apply_event, load_project_state},
        project::ProjectState,
        review::{handle_pr_merge, handle_pr_open, handle_pr_poll},
    };
    use cfk_core::{
        state_machine::{
            review::ReviewSlicePhase,
            work_item::{WorkItem, WorkItemStatus},
        },
        types::{
            forge::{CiStatus, PrComment, PrPollResult},
            ids::{LeaseId, ProjectId, WorkItemId},
            lease::{Lease, SessionIdentity},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
        },
    };
    use std::sync::Arc;

    fn test_project(root: &std::path::Path) -> ProjectState {
        ProjectState::new(ProjectId::new(), root.to_path_buf(), default_routing_table())
    }

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).unwrap();
        apply_event(state, &env.payload);
    }

    fn seed_review_item(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
    ) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Review,
            WorkType::PrCommentTriage, // will be overridden by the review flow
            "Open PR for add-command slice".to_string(),
        );
        let id = item.id.clone();
        append(root, seq, FactoryEvent::WorkItemAdded { work_item: item }, state);
        id
    }

    fn claim_review_item(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
        wid: &WorkItemId,
        session: &str,
    ) {
        let lease = Lease {
            id: LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: SessionIdentity::try_new(session.to_string()).expect("session"),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, seq, FactoryEvent::LeaseGranted { lease }, state);
    }

    // ── test: in-progress review item with no state → OpenPr step ────────────

    #[test]
    fn review_item_without_state_gets_open_pr_step() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut state = test_project(root);
        let mut seq = 0u64;

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let resp = handle_next_step(&state, None).expect("next_step");
        match resp {
            NextStepResponse::Ready(step) => {
                assert_eq!(step.work_item_id, wid);
                assert!(
                    matches!(step.action, StepAction::OpenPr { .. }),
                    "expected OpenPr, got {:?}", step.action
                );
            }
            NextStepResponse::Idle(_) => panic!("expected Ready"),
        }
    }

    // ── test: after PR opened → PrOpen phase → RunPrPoll step ────────────────

    #[tokio::test]
    async fn after_pr_open_next_step_is_run_pr_poll() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut state = test_project(root);
        let mut seq = 0u64;

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let forge = MemoryForge::new();
        let events = handle_pr_open(
            &state, &wid,
            "Add command".to_string(), "Implements add.".to_string(),
            "feature/add-command".to_string(), "main".to_string(),
            &(forge.clone() as Arc<dyn crate::forge::ForgeAdapter>),
        ).await.expect("open pr");

        for event in events {
            append(root, &mut seq, event, &mut state);
        }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::PrOpen)
        );
        assert_eq!(state.review_states.get(&wid).and_then(|r| r.pr_number), Some(1));

        let resp = handle_next_step(&state, None).expect("next_step");
        match resp {
            NextStepResponse::Ready(step) => {
                assert!(matches!(step.action, StepAction::RunPrPoll), "expected RunPrPoll");
            }
            NextStepResponse::Idle(_) => panic!("expected Ready for RunPrPoll"),
        }
    }

    // ── test: poll with comment → CommentTriagePending + triage items ─────────

    #[tokio::test]
    async fn poll_with_new_comment_creates_triage_item() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut state = test_project(root);
        let mut seq = 0u64;

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let forge = MemoryForge::new();
        let forge_dyn = forge.clone() as Arc<dyn crate::forge::ForgeAdapter>;

        // Open PR (PR number 1).
        let events = handle_pr_open(
            &state, &wid,
            "title".to_string(), "body".to_string(),
            "head".to_string(), "main".to_string(),
            &forge_dyn,
        ).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        // Set poll script: one comment, then all-green.
        forge.set_poll_script(1, PollScript::new(vec![
            PrPollResult {
                ci_status: CiStatus::Pending,
                approved: false,
                comments: vec![PrComment {
                    id: "c1".to_string(),
                    body: "Please add a doc comment.".to_string(),
                    author: "reviewer".to_string(),
                }],
            },
        ]));

        let events = handle_pr_poll(&state, &wid, &forge_dyn).await.unwrap();

        // Should produce a ReviewCommentTriageCreated event.
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], FactoryEvent::ReviewCommentTriageCreated { .. }));

        for event in &events { append(root, &mut seq, event.clone(), &mut state); }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::CommentTriagePending)
        );
        assert_eq!(state.review_states.get(&wid).map(|r| r.pending_triage.len()), Some(1));
    }

    // ── test: after triage comment posted → back to PrOpen ───────────────────

    #[tokio::test]
    async fn after_triage_posted_phase_returns_to_pr_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut state = test_project(root);
        let mut seq = 0u64;

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let forge = MemoryForge::new();
        let forge_dyn = forge.clone() as Arc<dyn crate::forge::ForgeAdapter>;

        // Open PR.
        let events = handle_pr_open(
            &state, &wid,
            "title".to_string(), "body".to_string(),
            "head".to_string(), "main".to_string(),
            &forge_dyn,
        ).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        // Poll: one comment.
        forge.set_poll_script(1, PollScript::new(vec![
            PrPollResult {
                ci_status: CiStatus::Pending,
                approved: false,
                comments: vec![PrComment {
                    id: "c1".to_string(),
                    body: "LGTM but rename the variable.".to_string(),
                    author: "alice".to_string(),
                }],
            },
        ]));
        let events = handle_pr_poll(&state, &wid, &forge_dyn).await.unwrap();
        let triage_item_id = if let FactoryEvent::ReviewCommentTriageCreated { triage_item_id, .. } = &events[0] {
            triage_item_id.clone()
        } else { panic!("expected ReviewCommentTriageCreated") };

        // Also add the triage work item.
        let triage_item = WorkItem::new(
            triage_item_id.clone(),
            PhaseKind::Review,
            WorkType::PrCommentTriage,
            "Comment c1: LGTM but rename the variable.".to_string(),
        );
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: triage_item }, &mut state);
        for event in events { append(root, &mut seq, event, &mut state); }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::CommentTriagePending)
        );

        // Post the review comment (triage done).
        append(root, &mut seq, FactoryEvent::ReviewCommentPosted {
            review_work_item_id: wid.clone(),
            comment_id: "c1".to_string(),
            triage_item_id: triage_item_id.clone(),
        }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: triage_item_id }, &mut state);

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::PrOpen),
            "after all triage done, should return to PrOpen"
        );
    }

    // ── test: all-green poll → AllGreen → MergePr step ───────────────────────

    #[tokio::test]
    async fn all_green_poll_produces_merge_pr_step() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut state = test_project(root);
        let mut seq = 0u64;

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let forge = MemoryForge::new();
        let forge_dyn = forge.clone() as Arc<dyn crate::forge::ForgeAdapter>;

        // Open PR.
        let events = handle_pr_open(
            &state, &wid,
            "title".to_string(), "body".to_string(),
            "head".to_string(), "main".to_string(),
            &forge_dyn,
        ).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        // Poll: all-green (no comments).
        forge.set_poll_script(1, PollScript::new(vec![
            PrPollResult {
                ci_status: CiStatus::Passing,
                approved: true,
                comments: vec![],
            },
        ]));

        let events = handle_pr_poll(&state, &wid, &forge_dyn).await.unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], FactoryEvent::ReviewAllGreen { .. }));

        for event in events { append(root, &mut seq, event, &mut state); }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::AllGreen)
        );

        // cf_next_step should now return MergePr.
        let resp = handle_next_step(&state, None).expect("next_step");
        match resp {
            NextStepResponse::Ready(step) => {
                assert!(matches!(step.action, StepAction::MergePr), "expected MergePr");
            }
            NextStepResponse::Idle(_) => panic!("expected Ready for MergePr"),
        }
    }

    // ── test: merge → Merged → work item Done ────────────────────────────────

    #[tokio::test]
    async fn merge_pr_marks_work_item_done() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let mut state = test_project(root);
        let mut seq = 0u64;

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let forge = MemoryForge::new();
        let forge_dyn = forge.clone() as Arc<dyn crate::forge::ForgeAdapter>;

        // Open PR.
        let events = handle_pr_open(
            &state, &wid, "t".to_string(), "b".to_string(),
            "h".to_string(), "main".to_string(), &forge_dyn,
        ).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        // Poll: all-green.
        forge.set_poll_script(1, PollScript::new(vec![
            PrPollResult { ci_status: CiStatus::Passing, approved: true, comments: vec![] }
        ]));
        let events = handle_pr_poll(&state, &wid, &forge_dyn).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        // Merge.
        let events = handle_pr_merge(&state, &wid, &forge_dyn).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        assert!(forge.is_merged(1), "PR should be merged on the forge");
        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::Merged)
        );
        let item = state.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(item.status, WorkItemStatus::Done, "work item must be Done after merge");
    }

    // ── test: restart durability for review phase ─────────────────────────────

    #[tokio::test]
    async fn review_state_survives_restart_via_event_replay() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let project_id = ProjectId::new();
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        let mut seq = 0u64;

        append(root, &mut seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);

        let wid = seed_review_item(root, &mut state, &mut seq);
        claim_review_item(root, &mut state, &mut seq, &wid, "conductor");

        let forge = MemoryForge::new();
        let forge_dyn = forge.clone() as Arc<dyn crate::forge::ForgeAdapter>;

        // Open PR.
        let events = handle_pr_open(
            &state, &wid, "title".to_string(), "body".to_string(),
            "feature/add".to_string(), "main".to_string(), &forge_dyn,
        ).await.unwrap();
        for event in events { append(root, &mut seq, event, &mut state); }

        // ── Simulate restart ──
        drop(state);
        let state = load_project_state(root).expect("load").expect("Some");

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::PrOpen),
            "review phase must survive restart"
        );
        assert_eq!(
            state.review_states.get(&wid).and_then(|r| r.pr_number),
            Some(1),
            "pr_number must survive restart"
        );

        // After restart, next_step should still return RunPrPoll.
        let resp = handle_next_step(&state, None).expect("next_step");
        match resp {
            NextStepResponse::Ready(step) => {
                assert!(matches!(step.action, StepAction::RunPrPoll));
            }
            NextStepResponse::Idle(_) => panic!("expected Ready after restart"),
        }
    }
}

// ── M4 exit-criterion test ────────────────────────────────────────────────────
// Full review lifecycle: claim → OpenPr → RunPrPoll (planted comment) →
// triage item → post reply → RunPrPoll (all-green) → MergePr → Done.
// Includes a simulated restart mid-review to verify event-replay durability.

#[cfg(test)]
mod m4_review_lifecycle {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        forge::{MemoryForge, PollScript},
        loader::{apply_event, load_project_state},
        project::ProjectState,
        review::{handle_pr_merge, handle_pr_open, handle_pr_poll},
    };
    use cfk_core::{
        state_machine::{
            review::ReviewSlicePhase,
            work_item::{WorkItem, WorkItemStatus},
        },
        types::{
            forge::{CiStatus, PrComment, PrPollResult},
            ids::{LeaseId, ProjectId, WorkItemId},
            lease::{Lease, SessionIdentity},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
        },
    };
    use std::sync::Arc;

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).unwrap();
        apply_event(state, &env.payload);
    }

    #[allow(clippy::too_many_lines)]
    #[tokio::test]
    async fn full_review_lifecycle_with_planted_comment_and_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let project_id = ProjectId::new();
        let mut state = ProjectState::new(
            project_id.clone(), root.to_path_buf(), default_routing_table()
        );
        let mut seq = 0u64;

        append(root, &mut seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);

        // ── Seed review work item ─────────────────────────────────────────────
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Review,
            WorkType::PrCommentTriage,
            "Open PR for add-command slice in toy-product".to_string(),
        );
        let wid = item.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: item }, &mut state);

        // ── Claim ─────────────────────────────────────────────────────────────
        let lease = Lease {
            id: LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: SessionIdentity::try_new("conductor-1".to_string()).unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        // ── cf_next_step → OpenPr ─────────────────────────────────────────────
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for OpenPr"),
        };
        assert!(matches!(step.action, StepAction::OpenPr { .. }), "first step must be OpenPr");

        // ── Forge: open PR (memory forge assigns PR #1) ───────────────────────
        let forge = MemoryForge::new();
        let forge_dyn = forge.clone() as Arc<dyn crate::forge::ForgeAdapter>;

        // Pre-load poll script:
        //   first poll: CI pending + one planted review comment
        //   second poll: CI passing, approved, no new comments
        forge.set_poll_script(1, PollScript::new(vec![
            PrPollResult {
                ci_status: CiStatus::Pending,
                approved: false,
                comments: vec![PrComment {
                    id: "planted-1".to_string(),
                    body: "Please add inline docs to the public function.".to_string(),
                    author: "human-reviewer".to_string(),
                }],
            },
            PrPollResult {
                ci_status: CiStatus::Passing,
                approved: true,
                comments: vec![PrComment {
                    id: "planted-1".to_string(),
                    body: "Please add inline docs to the public function.".to_string(),
                    author: "human-reviewer".to_string(),
                }],
            },
        ]));

        let events = handle_pr_open(
            &state, &wid,
            "feat(add-command): implement Add command and AdditionPerformed event".to_string(),
            "Closes the add-command slice.\n\nImplements the Add command handler.".to_string(),
            "feature/add-command".to_string(),
            "main".to_string(),
            &forge_dyn,
        ).await.expect("open_pr");

        for event in events { append(root, &mut seq, event, &mut state); }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::PrOpen)
        );

        // ── RESTART mid-review ────────────────────────────────────────────────
        drop(state);
        let mut state = load_project_state(root).expect("load").expect("Some");
        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::PrOpen),
            "PrOpen phase must survive restart"
        );

        // ── cf_next_step → RunPrPoll ──────────────────────────────────────────
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for RunPrPoll"),
        };
        assert!(matches!(step.action, StepAction::RunPrPoll), "after restart must be RunPrPoll");

        // ── First poll: planted comment ───────────────────────────────────────
        let events = handle_pr_poll(&state, &wid, &forge_dyn).await.expect("poll 1");
        assert_eq!(events.len(), 1, "one comment → one triage event");
        let triage_item_id = match &events[0] {
            FactoryEvent::ReviewCommentTriageCreated { triage_item_id, .. } => triage_item_id.clone(),
            other => panic!("expected ReviewCommentTriageCreated, got {other:?}"),
        };

        // Kernel creates the triage work item.
        let triage_item = WorkItem::new(
            triage_item_id.clone(),
            PhaseKind::Review,
            WorkType::PrCommentTriage,
            "Comment planted-1: Please add inline docs to the public function.".to_string(),
        );
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: triage_item }, &mut state);
        for event in events { append(root, &mut seq, event, &mut state); }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::CommentTriagePending)
        );

        // ── cf_next_step → SpawnAgent for triage ─────────────────────────────
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for triage SpawnAgent"),
        };
        assert!(
            matches!(step.action, StepAction::SpawnAgent { .. }),
            "triage step must be SpawnAgent, got {:?}", step.action
        );
        assert_eq!(step.work_item_id, wid, "step belongs to parent review item");

        // ── Agent posts a reply — kernel records ReviewCommentPosted ─────────
        forge_dyn.post_comment(1, "Done — added doc comment as requested.").await.unwrap();

        append(root, &mut seq, FactoryEvent::ReviewCommentPosted {
            review_work_item_id: wid.clone(),
            comment_id: "planted-1".to_string(),
            triage_item_id: triage_item_id.clone(),
        }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: triage_item_id }, &mut state);

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::PrOpen),
            "after triage done, back to PrOpen"
        );
        assert_eq!(
            state.review_states.get(&wid).map(|r| r.pending_triage.len()),
            Some(0)
        );

        // ── Second poll: all-green ────────────────────────────────────────────
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for second RunPrPoll"),
        };
        assert!(matches!(step.action, StepAction::RunPrPoll));

        let events = handle_pr_poll(&state, &wid, &forge_dyn).await.expect("poll 2");
        assert_eq!(events.len(), 1, "all-green poll produces ReviewAllGreen");
        assert!(matches!(events[0], FactoryEvent::ReviewAllGreen { .. }));

        for event in events { append(root, &mut seq, event, &mut state); }

        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::AllGreen)
        );

        // ── cf_next_step → MergePr ────────────────────────────────────────────
        let step = match handle_next_step(&state, None).expect("next_step") {
            NextStepResponse::Ready(s) => s,
            NextStepResponse::Idle(_) => panic!("expected Ready for MergePr"),
        };
        assert!(matches!(step.action, StepAction::MergePr), "must be MergePr");

        // ── Merge ─────────────────────────────────────────────────────────────
        let events = handle_pr_merge(&state, &wid, &forge_dyn).await.expect("merge");
        for event in events { append(root, &mut seq, event, &mut state); }

        assert!(forge.is_merged(1), "PR must be merged on the forge");
        assert_eq!(
            state.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::Merged)
        );

        let item = state.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(item.status, WorkItemStatus::Done, "slice must be Done after merge");

        // ── No more work ──────────────────────────────────────────────────────
        let response = handle_next_step(&state, None).expect("next_step after done");
        assert!(
            matches!(response, NextStepResponse::Idle(_)),
            "no work after merge"
        );

        // ── Final restart durability check ────────────────────────────────────
        let replayed = load_project_state(root).expect("replay").expect("Some");
        let replayed_item = replayed.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(replayed_item.status, WorkItemStatus::Done, "Done status survives replay");
        assert_eq!(
            replayed.review_states.get(&wid).map(|r| &r.phase),
            Some(&ReviewSlicePhase::Merged),
            "Merged phase survives replay"
        );
    }
}

/// M3 exit-criterion test.
///
/// Demonstrates the full M3 scenario: emc-verified workflow slices are ingested
/// into the kernel backlog and the first slice proceeds through the M2 TDD
/// machinery to completion.
#[cfg(test)]
mod m3_emc_integration {
    use crate::{
        config::default_routing_table,
        emc::read_verified_slices,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::work_item::{WorkItem, WorkItemStatus},
        types::{
            gate::{GateKind, GateVerdict},
            ids::{ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
            tdd::TddPhase,
        },
    };
    use std::fs;

    fn setup_toy_product_model(root: &std::path::Path) {
        let dir = root.join("model").join("events").join("v1");
        fs::create_dir_all(&dir).unwrap();

        let write = |name: &str, payload: serde_json::Value| {
            fs::write(dir.join(name), serde_json::to_string_pretty(&payload).unwrap()).unwrap();
        };

        write("0000000001-wf.json", serde_json::json!({
            "schema_version": "1", "event_id": "e1", "stream_id": "m",
            "type": "WorkflowAdded",
            "payload": { "workflow": "addition", "name": "Addition", "description": "Adds numbers." }
        }));
        write("0000000002-s1.json", serde_json::json!({
            "schema_version": "1", "event_id": "e2", "stream_id": "m",
            "type": "SliceAdded",
            "payload": {
                "workflow": "addition", "slug": "add-command",
                "name": "Add Command", "kind": "state_change",
                "description": "Implement the add function: takes two u64, returns their sum."
            }
        }));
        write("0000000003-s2.json", serde_json::json!({
            "schema_version": "1", "event_id": "e3", "stream_id": "m",
            "type": "SliceAdded",
            "payload": {
                "workflow": "addition", "slug": "sum-view",
                "name": "Sum View", "kind": "state_view",
                "description": "Project AdditionPerformed into a SumResult read model."
            }
        }));
        write("0000000004-rd.json", serde_json::json!({
            "schema_version": "1", "event_id": "e4", "stream_id": "m",
            "type": "WorkflowReadinessDeclared",
            "payload": { "workflow": "addition" }
        }));
    }

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).unwrap();
        apply_event(state, &env.payload);
    }

    #[test]
    fn emc_slices_appear_in_backlog_and_first_slice_completes_tdd_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        setup_toy_product_model(root);

        // ── Ingest slices from verified emc model ──────────────────────────────
        let slices = read_verified_slices(root).unwrap();
        assert_eq!(slices.len(), 2, "both slices of the verified addition workflow");

        let project_id = ProjectId::new();
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        let mut seq = 0u64;

        // Emit ProjectInitialized so load_project_state can replay.
        append(root, &mut seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);

        let existing_slugs: std::collections::HashSet<String> = state
            .work_items.iter().filter_map(|i| i.emc_slug.clone()).collect();

        let mut work_item_ids: Vec<WorkItemId> = Vec::new();
        for slice in &slices {
            if existing_slugs.contains(&slice.slug) {
                continue;
            }
            let work_type = if slice.kind == "translation" {
                WorkType::MechanicalTransform
            } else {
                WorkType::NarrowestStepImplementation
            };
            let item = WorkItem::from_emc_slice(
                WorkItemId::new(), PhaseKind::Development, work_type,
                format!("[{}] {}", slice.slug, slice.name),
                slice.slug.clone(),
            );
            let id = item.id.clone();
            append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: item }, &mut state);
            work_item_ids.push(id);
        }

        assert_eq!(state.work_items.len(), 2);
        assert!(
            state.work_items.iter().all(|i| i.emc_slug.is_some()),
            "every ingested item carries its emc_slug"
        );

        // ── Build the first slice through the M2 TDD machinery ────────────────
        let wid = work_item_ids[0].clone();

        // Claim and start TDD.
        append(root, &mut seq, FactoryEvent::LeaseGranted {
            lease: cfk_core::types::lease::Lease {
                id: cfk_core::types::ids::LeaseId::new(),
                work_item_id: wid.clone(),
                session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                    "alice".to_string()).unwrap(),
                granted_at: chrono::Utc::now(),
                expires_at: None,
            }
        }, &mut state);
        append(root, &mut seq, FactoryEvent::TddSliceStarted {
            work_item_id: wid.clone(), author_identity: "alice".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::WriteTest));

        // WriteTest → TestReviewGate → RedCheck → Implement → CheckProgress (green) → ImplReviewGate → LintCheck → Done
        append(root, &mut seq, FactoryEvent::TddTestSubmitted {
            work_item_id: wid.clone(), frame_depth: 0,
            test_content: "assert_eq!(add(2, 2), 4);".to_string(),
            author_identity: "alice".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::TestReviewGate));

        // Gate: approved.
        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(), gate_kind: GateKind::TestReview,
            verdict: GateVerdict::Approved, reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::RedCheck));

        // Red check: fails (expected).
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(), check_name: "tests".to_string(),
            passed: false, first_error: Some("error[E0425]: cannot find function `add`".to_string()),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::Implement));

        // Implement → CheckProgress.
        append(root, &mut seq, FactoryEvent::TddPhaseAdvanced {
            work_item_id: wid.clone(), frame_depth: 0, new_phase: TddPhase::CheckProgress,
        }, &mut state);

        // Green.
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(), check_name: "tests".to_string(),
            passed: true, first_error: None,
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::ImplReviewGate));

        // Implementation review: approved.
        append(root, &mut seq, FactoryEvent::TddGateVerdict {
            work_item_id: wid.clone(), gate_kind: GateKind::ImplementationReview,
            verdict: GateVerdict::Approved, reviewer_id: "bob".to_string(),
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::LintCheck));

        // Lint passes → Done.
        append(root, &mut seq, FactoryEvent::TddCheckResult {
            work_item_id: wid.clone(), check_name: "lint".to_string(),
            passed: true, first_error: None,
        }, &mut state);
        assert_eq!(state.dev_states.get(&wid).and_then(|d| d.current_phase()), Some(&TddPhase::Done));

        append(root, &mut seq, FactoryEvent::TddSliceDone { work_item_id: wid.clone() }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: wid.clone() }, &mut state);

        let item = state.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(item.status, WorkItemStatus::Done, "emc-ingested slice must reach Done via M2 TDD machinery");

        // Second slice is still ready.
        let wid2 = work_item_ids[1].clone();
        let item2 = state.work_items.iter().find(|i| i.id == wid2).unwrap();
        assert_eq!(item2.status, WorkItemStatus::Ready);

        // Restart durability: replay events and verify the completed slice is Done.
        let replayed = load_project_state(root).unwrap().unwrap();
        let replayed_item = replayed.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(replayed_item.status, WorkItemStatus::Done, "M3 completed slice survives restart");
        assert_eq!(replayed_item.emc_slug.as_deref(), Some("add-command"), "emc_slug preserved through replay");
    }
}

// ── M5: Discovery phase ──────────────────────────────────────────────────────

#[cfg(test)]
mod discovery_phase {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::{
            discovery::DiscoveryPhase,
            work_item::{WorkItem, WorkItemStatus},
        },
        types::{
            ids::{ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
        },
    };

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).expect("append");
        apply_event(state, &env.payload);
    }

    fn test_project(root: &std::path::Path, seq: &mut u64) -> ProjectState {
        let project_id = ProjectId::new();
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        append(root, seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);
        state
    }

    fn seed_discovery_item(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
    ) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Discovery,
            WorkType::SocraticDiscovery,
            "Discover the inventory management product.".to_string(),
        );
        let id = item.id.clone();
        append(root, seq, FactoryEvent::WorkItemAdded { work_item: item }, state);
        id
    }

    #[test]
    fn next_step_for_unclaimed_discovery_item_is_ready() {
        let dir = tempfile::tempdir().unwrap();
        let mut seq = 0;
        let mut state = test_project(dir.path(), &mut seq);
        seed_discovery_item(dir.path(), &mut state, &mut seq);

        let resp = handle_next_step(&state, None).unwrap();
        assert!(matches!(resp, NextStepResponse::Ready(_)));
    }

    #[test]
    fn dialogue_step_returned_for_in_progress_discovery_item() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_discovery_item(root, &mut state, &mut seq);

        // Claim the item.
        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert_eq!(step.work_item_id, wid);
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }));
    }

    #[test]
    fn brief_ready_step_is_ask_human() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_discovery_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        // Agent submits brief.
        append(root, &mut seq, FactoryEvent::DiscoveryBriefDrafted {
            work_item_id: wid.clone(),
            brief_content: "A brief for inventory management.".to_string(),
            workflows: vec!["receive stock".to_string(), "pick and ship".to_string()],
        }, &mut state);

        assert_eq!(
            state.discovery_states.get(&wid).map(|d| &d.phase),
            Some(&DiscoveryPhase::BriefReady),
        );

        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert!(matches!(step.action, StepAction::AskHuman { .. }));
    }

    #[test]
    fn approved_discovery_queues_workflows_and_completes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_discovery_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);
        append(root, &mut seq, FactoryEvent::DiscoveryBriefDrafted {
            work_item_id: wid.clone(),
            brief_content: "Inventory management brief.".to_string(),
            workflows: vec!["receive stock".to_string(), "pick and ship".to_string()],
        }, &mut state);
        append(root, &mut seq, FactoryEvent::DiscoveryApproved { work_item_id: wid.clone() }, &mut state);

        // Queue two event-modeling work items.
        let em1 = WorkItem::new(WorkItemId::new(), PhaseKind::EventModeling, WorkType::EventModelAuthoring, "Model workflow: receive stock".to_string());
        let em2 = WorkItem::new(WorkItemId::new(), PhaseKind::EventModeling, WorkType::EventModelAuthoring, "Model workflow: pick and ship".to_string());
        let em1_id = em1.id.clone();
        let em2_id = em2.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: em1 }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: em2 }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: wid.clone() }, &mut state);

        let disc_item = state.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(disc_item.status, WorkItemStatus::Done, "discovery item must be completed");

        let em1_item = state.work_items.iter().find(|i| i.id == em1_id).unwrap();
        assert_eq!(em1_item.phase, PhaseKind::EventModeling, "first workflow must be queued");
        let em2_item = state.work_items.iter().find(|i| i.id == em2_id).unwrap();
        assert_eq!(em2_item.phase, PhaseKind::EventModeling, "second workflow must be queued");

        // Restart durability.
        let replayed = load_project_state(root).unwrap().unwrap();
        let replayed_disc = replayed.work_items.iter().find(|i| i.id == wid).unwrap();
        assert_eq!(replayed_disc.status, WorkItemStatus::Done, "discovery done survives restart");
    }
}

// ── M5: Architecture phase ───────────────────────────────────────────────────

#[cfg(test)]
mod architecture_phase {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::{
            architecture::{AdrPhase},
            work_item::{WorkItem, WorkItemStatus},
        },
        types::{
            architecture::AdrStatus,
            gate::{GateKind, GateVerdict, VetoReason},
            ids::{AdrId, ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
        },
    };

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).expect("append");
        apply_event(state, &env.payload);
    }

    fn test_project(root: &std::path::Path, seq: &mut u64) -> ProjectState {
        let project_id = ProjectId::new();
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        append(root, seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);
        state
    }

    fn seed_adr_item(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
    ) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Architecture,
            WorkType::AdrDrafting,
            "Decide on event store technology.".to_string(),
        );
        let id = item.id.clone();
        append(root, seq, FactoryEvent::WorkItemAdded { work_item: item }, state);
        id
    }

    #[test]
    fn drafting_step_returned_for_in_progress_adr_item() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_adr_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }), "drafting must be SpawnAgent");
    }

    #[test]
    fn pending_review_step_is_gate_review() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_adr_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let adr_id = AdrId::new();
        append(root, &mut seq, FactoryEvent::AdrDrafted {
            work_item_id: wid.clone(),
            adr_id: adr_id.clone(),
            title: "Use SQLite for event store".to_string(),
            content: "Context: ...\nDecision: SQLite\nConsequences: ...".to_string(),
        }, &mut state);

        assert_eq!(
            state.adr_states.get(&wid).map(|a| &a.phase),
            Some(&AdrPhase::PendingReview),
        );
        assert_eq!(state.adrs.len(), 1);

        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert!(
            matches!(step.action, StepAction::GateReview { gate_kind: GateKind::AdrReview, .. }),
            "pending review must be GateReview(AdrReview)"
        );
    }

    #[test]
    fn accepted_adr_completes_work_item_and_updates_registry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_adr_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let adr_id = AdrId::new();
        append(root, &mut seq, FactoryEvent::AdrDrafted {
            work_item_id: wid.clone(),
            adr_id: adr_id.clone(),
            title: "Use SQLite for event store".to_string(),
            content: "Context: ...\nDecision: SQLite\nConsequences: ...".to_string(),
        }, &mut state);
        append(root, &mut seq, FactoryEvent::AdrDecided {
            work_item_id: wid.clone(),
            adr_id: adr_id.clone(),
            accepted: true,
            reason: None,
        }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: wid.clone() }, &mut state);

        assert_eq!(state.adr_states.get(&wid).map(|a| &a.phase), Some(&AdrPhase::Accepted));
        assert_eq!(state.adrs[0].status, AdrStatus::Accepted);
        assert_eq!(
            state.work_items.iter().find(|i| i.id == wid).map(|i| &i.status),
            Some(&WorkItemStatus::Done),
        );

        // Restart durability.
        let replayed = load_project_state(root).unwrap().unwrap();
        assert_eq!(replayed.adrs[0].status, AdrStatus::Accepted, "accepted ADR survives restart");
    }

    #[test]
    fn vetoed_adr_marks_rejected_in_registry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_adr_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let adr_id = AdrId::new();
        append(root, &mut seq, FactoryEvent::AdrDrafted {
            work_item_id: wid.clone(),
            adr_id: adr_id.clone(),
            title: "Use raw SQL everywhere".to_string(),
            content: "Context: ...\nDecision: raw SQL\nConsequences: ...".to_string(),
        }, &mut state);
        append(root, &mut seq, FactoryEvent::AdrDecided {
            work_item_id: wid.clone(),
            adr_id: adr_id.clone(),
            accepted: false,
            reason: Some("Contradicts event-sourcing constraint.".to_string()),
        }, &mut state);

        assert_eq!(state.adr_states.get(&wid).map(|a| &a.phase), Some(&AdrPhase::Rejected));
        assert_eq!(state.adrs[0].status, AdrStatus::Rejected);
    }

    /// Suppresses unused-import warnings for types used only as match patterns.
    #[allow(dead_code)]
    fn _use_types() {
        let _ = GateVerdict::Approved;
        let _ = VetoReason::try_new("x".to_string());
    }
}

// ── M5: Design-system phase ──────────────────────────────────────────────────

#[cfg(test)]
mod design_system_phase {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::{
            design::DesignPhase,
            work_item::{WorkItem, WorkItemStatus},
        },
        types::{
            design::AtomicKind,
            ids::{ComponentId, ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
        },
    };

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).expect("append");
        apply_event(state, &env.payload);
    }

    fn test_project(root: &std::path::Path, seq: &mut u64) -> ProjectState {
        let project_id = ProjectId::new();
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        append(root, seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);
        state
    }

    fn seed_design_item(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
    ) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::DesignSystem,
            WorkType::DesignSystemBuild,
            "Design component for workflow: receive stock".to_string(),
        );
        let id = item.id.clone();
        append(root, seq, FactoryEvent::WorkItemAdded { work_item: item }, state);
        id
    }

    #[test]
    fn building_step_returned_for_in_progress_design_item() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_design_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }), "building must be SpawnAgent");
    }

    #[test]
    fn component_added_updates_inventory_and_completes_item() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);
        let wid = seed_design_item(root, &mut state, &mut seq);

        let lease = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new(
                "alice".to_string(),
            )
            .unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease }, &mut state);

        let component_id = ComponentId::new();
        append(root, &mut seq, FactoryEvent::DesignComponentAdded {
            work_item_id: wid.clone(),
            component_id: component_id.clone(),
            name: "ReceiveStockPage".to_string(),
            kind: AtomicKind::Page,
            slice_ref: None,
        }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: wid.clone() }, &mut state);

        assert_eq!(
            state.design_states.get(&wid).map(|d| &d.phase),
            Some(&DesignPhase::Done),
        );
        assert_eq!(state.design_inventory.len(), 1);
        assert_eq!(state.design_inventory[0].name, "ReceiveStockPage");
        assert_eq!(state.design_inventory[0].kind, AtomicKind::Page);
        assert_eq!(
            state.work_items.iter().find(|i| i.id == wid).map(|i| &i.status),
            Some(&WorkItemStatus::Done),
        );

        // Restart durability.
        let replayed = load_project_state(root).unwrap().unwrap();
        assert_eq!(replayed.design_inventory.len(), 1, "component inventory survives restart");
        assert_eq!(replayed.design_inventory[0].name, "ReceiveStockPage");
    }

    #[test]
    fn cross_check_generates_items_for_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);

        // Pre-populate: one component for "receive stock" workflow.
        let comp_id = ComponentId::new();
        let pre_existing_item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::DesignSystem,
            WorkType::DesignSystemBuild,
            "existing".to_string(),
        );
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: pre_existing_item.clone() }, &mut state);
        append(root, &mut seq, FactoryEvent::DesignComponentAdded {
            work_item_id: pre_existing_item.id.clone(),
            component_id: comp_id.clone(),
            name: "receive stock Page".to_string(),
            kind: AtomicKind::Page,
            slice_ref: None,
        }, &mut state);

        // Cross-check: two workflows, one already covered.
        let gap_item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::DesignSystem,
            WorkType::DesignSystemBuild,
            "Design component for workflow: pick and ship".to_string(),
        );
        let gap_id = gap_item.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: gap_item }, &mut state);
        append(root, &mut seq, FactoryEvent::DesignCrossCheckCompleted {
            generated_item_ids: vec![gap_id.clone()],
        }, &mut state);

        // Only the gap item was created; inventory still has 1 entry.
        let design_items: Vec<_> = state.work_items.iter()
            .filter(|i| i.phase == PhaseKind::DesignSystem && i.id != pre_existing_item.id)
            .collect();
        assert_eq!(design_items.len(), 1, "one gap item created for pick-and-ship");
        assert_eq!(design_items[0].id, gap_id);
    }
}

// ── M5 exit criterion ────────────────────────────────────────────────────────

#[cfg(test)]
mod m5_exit_criterion {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::work_item::{WorkItem, WorkItemStatus},
        types::{
            architecture::AdrStatus,
            design::AtomicKind,
            ids::{AdrId, ComponentId, ProjectId, WorkItemId},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
        },
    };

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).expect("append");
        apply_event(state, &env.payload);
    }

    /// M5 exit criterion: each of the three new phases (Discovery, Architecture,
    /// Design-system) runs individually on toy product data, with restart
    /// durability verified for each.
    #[allow(clippy::too_many_lines)]
    #[test]
    fn all_three_phases_run_individually_on_toy_product() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let project_id = ProjectId::new();
        let mut state = ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        let mut seq = 0u64;
        append(root, &mut seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);

        // ── 1. Discovery phase ───────────────────────────────────────────

        let disc_item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Discovery,
            WorkType::SocraticDiscovery,
            "Discover toy product.".to_string(),
        );
        let disc_id = disc_item.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: disc_item }, &mut state);

        let lease_disc = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: disc_id.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new("alice".to_string()).unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease: lease_disc }, &mut state);

        // cf_next_step → SpawnAgent(SocraticDiscovery)
        let resp = handle_next_step(&state, Some(PhaseKind::Discovery)).unwrap();
        assert!(matches!(resp, NextStepResponse::Ready(ref s) if matches!(s.action, StepAction::SpawnAgent { .. })));

        // Agent submits brief.
        append(root, &mut seq, FactoryEvent::DiscoveryBriefDrafted {
            work_item_id: disc_id.clone(),
            brief_content: "Toy product manages widgets.".to_string(),
            workflows: vec!["add widget".to_string(), "remove widget".to_string()],
        }, &mut state);

        // cf_next_step → AskHuman
        let resp = handle_next_step(&state, Some(PhaseKind::Discovery)).unwrap();
        assert!(matches!(resp, NextStepResponse::Ready(ref s) if matches!(s.action, StepAction::AskHuman { .. })));

        // Human approves → workflows queued.
        append(root, &mut seq, FactoryEvent::DiscoveryApproved { work_item_id: disc_id.clone() }, &mut state);
        let em1 = WorkItem::new(WorkItemId::new(), PhaseKind::EventModeling, WorkType::EventModelAuthoring, "Model workflow: add widget".to_string());
        let em2 = WorkItem::new(WorkItemId::new(), PhaseKind::EventModeling, WorkType::EventModelAuthoring, "Model workflow: remove widget".to_string());
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: em1 }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: em2 }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: disc_id.clone() }, &mut state);

        assert_eq!(
            state.work_items.iter().find(|i| i.id == disc_id).map(|i| &i.status),
            Some(&WorkItemStatus::Done),
            "discovery done",
        );
        let em_items: Vec<_> = state.work_items.iter().filter(|i| i.phase == PhaseKind::EventModeling).collect();
        assert_eq!(em_items.len(), 2, "two event-modeling items queued");

        // Restart durability for discovery.
        let replayed = load_project_state(root).unwrap().unwrap();
        assert_eq!(
            replayed.work_items.iter().find(|i| i.id == disc_id).map(|i| &i.status),
            Some(&WorkItemStatus::Done),
            "discovery survives restart",
        );

        // ── 2. Architecture phase ────────────────────────────────────────

        let adr_item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Architecture,
            WorkType::AdrDrafting,
            "Decide widget store technology.".to_string(),
        );
        let adr_wid = adr_item.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: adr_item }, &mut state);

        let lease_adr = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: adr_wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new("alice".to_string()).unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease: lease_adr }, &mut state);

        // cf_next_step → SpawnAgent(AdrDrafting)
        let resp = handle_next_step(&state, Some(PhaseKind::Architecture)).unwrap();
        assert!(matches!(resp, NextStepResponse::Ready(ref s) if matches!(s.action, StepAction::SpawnAgent { .. })));

        // Agent submits draft.
        let adr_record_id = AdrId::new();
        append(root, &mut seq, FactoryEvent::AdrDrafted {
            work_item_id: adr_wid.clone(),
            adr_id: adr_record_id.clone(),
            title: "Use in-memory store for widgets".to_string(),
            content: "Context: toy product.\nDecision: in-memory.\nConsequences: simple.".to_string(),
        }, &mut state);

        // cf_next_step → GateReview(AdrReview)
        let resp = handle_next_step(&state, Some(PhaseKind::Architecture)).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert!(
            matches!(step.action, StepAction::GateReview { gate_kind: cfk_core::types::gate::GateKind::AdrReview, .. }),
            "ADR gate review step"
        );

        // Reviewer approves.
        append(root, &mut seq, FactoryEvent::AdrDecided {
            work_item_id: adr_wid.clone(),
            adr_id: adr_record_id.clone(),
            accepted: true,
            reason: None,
        }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: adr_wid.clone() }, &mut state);

        assert_eq!(state.adrs[0].status, AdrStatus::Accepted, "ADR accepted");

        // Restart durability for architecture.
        let replayed = load_project_state(root).unwrap().unwrap();
        assert_eq!(replayed.adrs[0].status, AdrStatus::Accepted, "ADR accepted survives restart");

        // ── 3. Design-system phase ───────────────────────────────────────

        let ds_item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::DesignSystem,
            WorkType::DesignSystemBuild,
            "Design widget list page.".to_string(),
        );
        let ds_wid = ds_item.id.clone();
        append(root, &mut seq, FactoryEvent::WorkItemAdded { work_item: ds_item }, &mut state);

        let lease_ds = cfk_core::types::lease::Lease {
            id: cfk_core::types::ids::LeaseId::new(),
            work_item_id: ds_wid.clone(),
            session_identity: cfk_core::types::lease::SessionIdentity::try_new("alice".to_string()).unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        };
        append(root, &mut seq, FactoryEvent::LeaseGranted { lease: lease_ds }, &mut state);

        // cf_next_step → SpawnAgent(DesignSystemBuild)
        let resp = handle_next_step(&state, Some(PhaseKind::DesignSystem)).unwrap();
        assert!(matches!(resp, NextStepResponse::Ready(ref s) if matches!(s.action, StepAction::SpawnAgent { .. })));

        // Agent adds component.
        let comp_id = ComponentId::new();
        append(root, &mut seq, FactoryEvent::DesignComponentAdded {
            work_item_id: ds_wid.clone(),
            component_id: comp_id.clone(),
            name: "WidgetListPage".to_string(),
            kind: AtomicKind::Page,
            slice_ref: Some("add-widget".to_string()),
        }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: ds_wid.clone() }, &mut state);

        assert_eq!(state.design_inventory.len(), 1);
        assert_eq!(state.design_inventory[0].name, "WidgetListPage");

        // Restart durability for design system.
        let replayed = load_project_state(root).unwrap().unwrap();
        assert_eq!(replayed.design_inventory.len(), 1, "design inventory survives restart");
        assert_eq!(replayed.design_inventory[0].name, "WidgetListPage");
        assert_eq!(
            replayed.work_items.iter().filter(|i| i.status == WorkItemStatus::Done).count(),
            3, // discovery + architecture + design = 3 done items
            "all three phases completed",
        );
    }
}

// ── M6: Walking skeleton — overlapping WIP across phases ────────────────────

#[cfg(test)]
mod m6_walking_skeleton {
    use crate::{
        commands::{handle_next_step, NextStepResponse},
        config::default_routing_table,
        events::{FactoryEvent, append_event},
        loader::{apply_event, load_project_state},
        project::ProjectState,
    };
    use cfk_core::{
        state_machine::{
            discovery::DiscoveryPhase,
            work_item::{WorkItem, WorkItemStatus},
        },
        types::{
            ids::{LeaseId, ProjectId, WorkItemId},
            lease::{Lease, SessionIdentity},
            phase::PhaseKind,
            routing::WorkType,
            step::StepAction,
            tdd::TddPhase,
        },
    };

    fn append(
        root: &std::path::Path,
        seq: &mut u64,
        event: FactoryEvent,
        state: &mut ProjectState,
    ) {
        *seq += 1;
        let env = append_event(root, *seq, event).expect("append");
        apply_event(state, &env.payload);
    }

    fn test_project(root: &std::path::Path, seq: &mut u64) -> ProjectState {
        let project_id = ProjectId::new();
        let mut state =
            ProjectState::new(project_id.clone(), root.to_path_buf(), default_routing_table());
        append(root, seq, FactoryEvent::ProjectInitialized { id: project_id }, &mut state);
        state
    }

    fn make_lease(wid: WorkItemId, identity: &str) -> Lease {
        Lease {
            id: LeaseId::new(),
            work_item_id: wid,
            session_identity: SessionIdentity::try_new(identity.to_string()).unwrap(),
            granted_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    /// Seed a dev item that is `InProgress` (TDD started, `WriteTest` phase).
    fn seed_dev_item_in_progress(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
    ) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Development,
            WorkType::OuterBehavioralTestWriting,
            "Implement add-widget slice.".to_string(),
        );
        let wid = item.id.clone();
        append(root, seq, FactoryEvent::WorkItemAdded { work_item: item }, state);
        append(root, seq, FactoryEvent::LeaseGranted { lease: make_lease(wid.clone(), "alice") }, state);
        append(root, seq, FactoryEvent::TddSliceStarted {
            work_item_id: wid.clone(),
            author_identity: "alice".to_string(),
        }, state);
        wid
    }

    /// Seed a discovery item that is `InProgress` (claimed, `Dialogue` phase).
    fn seed_discovery_item_in_progress(
        root: &std::path::Path,
        state: &mut ProjectState,
        seq: &mut u64,
    ) -> WorkItemId {
        let item = WorkItem::new(
            WorkItemId::new(),
            PhaseKind::Discovery,
            WorkType::SocraticDiscovery,
            "Discover the inventory management product.".to_string(),
        );
        let wid = item.id.clone();
        append(root, seq, FactoryEvent::WorkItemAdded { work_item: item }, state);
        append(root, seq, FactoryEvent::LeaseGranted { lease: make_lease(wid.clone(), "bob") }, state);
        wid
    }

    #[test]
    fn dev_takes_priority_over_discovery_when_no_filter() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);

        let dev_wid = seed_dev_item_in_progress(root, &mut state, &mut seq);
        seed_discovery_item_in_progress(root, &mut state, &mut seq);

        // Without a filter, dev phase has highest priority.
        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert_eq!(step.work_item_id, dev_wid, "dev item should be returned first");
        assert_eq!(step.phase, PhaseKind::Development);
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }));
    }

    #[test]
    fn phase_filter_scopes_to_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);

        seed_dev_item_in_progress(root, &mut state, &mut seq);
        let disc_wid = seed_discovery_item_in_progress(root, &mut state, &mut seq);

        // With Discovery filter, the discovery item is returned despite dev having priority globally.
        let resp = handle_next_step(&state, Some(PhaseKind::Discovery)).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert_eq!(step.work_item_id, disc_wid, "discovery filter must return discovery item");
        assert_eq!(step.phase, PhaseKind::Discovery);
        assert!(matches!(step.action, StepAction::SpawnAgent { .. }));
    }

    #[test]
    fn phase_filter_scopes_to_development() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);

        let dev_wid = seed_dev_item_in_progress(root, &mut state, &mut seq);
        seed_discovery_item_in_progress(root, &mut state, &mut seq);

        let resp = handle_next_step(&state, Some(PhaseKind::Development)).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready") };
        assert_eq!(step.work_item_id, dev_wid);
        assert_eq!(step.phase, PhaseKind::Development);
    }

    #[test]
    fn discovery_still_has_work_after_dev_item_completes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);

        let dev_wid = seed_dev_item_in_progress(root, &mut state, &mut seq);
        let disc_wid = seed_discovery_item_in_progress(root, &mut state, &mut seq);

        // Complete the dev item.
        append(root, &mut seq, FactoryEvent::TddSliceDone { work_item_id: dev_wid.clone() }, &mut state);
        append(root, &mut seq, FactoryEvent::WorkItemCompleted { work_item_id: dev_wid.clone() }, &mut state);

        assert_eq!(
            state.work_items.iter().find(|i| i.id == dev_wid).unwrap().status,
            WorkItemStatus::Done,
        );

        // Discovery item still has a step available.
        let resp = handle_next_step(&state, None).unwrap();
        let NextStepResponse::Ready(step) = resp else { panic!("expected Ready after dev done") };
        assert_eq!(step.work_item_id, disc_wid);
        assert_eq!(step.phase, PhaseKind::Discovery);
    }

    #[test]
    fn both_phase_states_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut seq = 0;
        let mut state = test_project(root, &mut seq);

        let dev_wid = seed_dev_item_in_progress(root, &mut state, &mut seq);
        let disc_wid = seed_discovery_item_in_progress(root, &mut state, &mut seq);

        // Advance discovery to BriefReady.
        append(root, &mut seq, FactoryEvent::DiscoveryBriefDrafted {
            work_item_id: disc_wid.clone(),
            brief_content: "Product brief content.".to_string(),
            workflows: vec!["add-widget".to_string()],
        }, &mut state);

        assert_eq!(
            state.discovery_states.get(&disc_wid).map(|d| d.phase.clone()),
            Some(DiscoveryPhase::BriefReady),
        );
        assert_eq!(
            state.dev_states.get(&dev_wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::WriteTest),
        );

        // Replay — both states must be restored.
        let replayed = load_project_state(root).unwrap().unwrap();
        assert_eq!(
            replayed.discovery_states.get(&disc_wid).map(|d| d.phase.clone()),
            Some(DiscoveryPhase::BriefReady),
            "discovery BriefReady survives restart",
        );
        assert_eq!(
            replayed.dev_states.get(&dev_wid).and_then(|d| d.current_phase()),
            Some(&TddPhase::WriteTest),
            "dev WriteTest phase survives restart",
        );

        // Phase filter still works on replayed state.
        let dev_resp = handle_next_step(&replayed, Some(PhaseKind::Development)).unwrap();
        assert!(matches!(dev_resp, NextStepResponse::Ready(_)), "dev still ready after restart");

        let disc_resp = handle_next_step(&replayed, Some(PhaseKind::Discovery)).unwrap();
        assert!(matches!(disc_resp, NextStepResponse::Ready(_)), "discovery still ready after restart");
    }
}
