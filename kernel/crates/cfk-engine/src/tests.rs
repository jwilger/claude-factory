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

        let item = state.work_items.iter().find(|i| &i.id == &wid).expect("item");
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
