//! Behavioral tests for cfk-engine.
//!
//! Tests follow the pattern: given events → when command → then result.
//! No I/O mocks — the event store uses a real temp directory.

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
