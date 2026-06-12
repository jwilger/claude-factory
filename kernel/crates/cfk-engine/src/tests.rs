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
